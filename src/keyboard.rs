use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::sync::Arc;

use axum::{headers, TypedHeader};
use axum::extract::{ConnectInfo, WebSocketUpgrade};
use axum::extract::ws::{Message, WebSocket};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use tokio::sync::Mutex;
use tokio::task::JoinSet;

use crate::DEVICE_CTX;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    println!("`{user_agent}` at {addr} connected.");
    ws.on_upgrade(move |socket| handle_socket(socket, addr))
}

async fn handle_socket(socket: WebSocket, who: SocketAddr) {
    let (sender, mut receiver) = socket.split();

    let mut join_set = JoinSet::new();

    join_set.spawn(async move {
        let sender = Arc::new(Mutex::new(sender));
        // 肯定不会是空，必然是在 DEVICE_CTX 初始化后才走到这
        let keyboard_receiver = Arc::new(Mutex::new(DEVICE_CTX.get().unwrap().keyboard_device.keyboard_update_sender.subscribe()));
        loop {
            let timeout_sender = sender.clone();

            let mut join_set = JoinSet::new();
            join_set.spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                if timeout_sender.lock().await.send(Message::Ping(vec![1, 2, 3])).await.is_ok() {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(())
                }
            });

            let keyboard_status_sender = sender.clone();
            let keyboard_receiver = keyboard_receiver.clone();
            join_set.spawn(async move {
                let mut keyboard_receiver = keyboard_receiver.lock().await;
                let mut keyboard_status_sender = keyboard_status_sender.lock().await;
                let keyboard_status = keyboard_receiver.borrow_and_update().to_vec();
                if keyboard_receiver.changed().await.is_ok() &&
                    keyboard_status_sender.send(Message::Binary(keyboard_status)).await.is_ok()
                {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(())
                }
            });

            if let Some(res) = join_set.join_next().await {
                // TODO 我不知道什么情况下会是 Err
                if res.unwrap().is_break() {
                    break;
                }
            }
            join_set.shutdown().await;
        }
    });

    join_set.spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if process_message(msg, who).is_break() {
                break;
            }
        }
    });

    let _ = join_set.join_next().await;
    join_set.shutdown().await;


    println!("Websocket context {} destroyed", who);
}

fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<(), ()> {
    match msg {
        Message::Binary(d) => {
            println!(">>> {} sent {} bytes: {:?}", who, d.len(), d);
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> {} sent close with code {} and reason `{}`",
                    who, cf.code, cf.reason
                );
            } else {
                println!(">>> {} somehow sent close message without CloseFrame", who);
            }
            return ControlFlow::Break(());
        }
        _ => {}
    }
    ControlFlow::Continue(())
}
