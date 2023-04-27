use std::{net::SocketAddr,
          ops::ControlFlow,
          sync::Arc,
          time::Duration};

use axum::{
    extract::{
        ConnectInfo,
        WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    headers,
    response::IntoResponse,
    TypedHeader,
};
use futures::{SinkExt, StreamExt};
use tokio::{sync::Mutex,
            task::JoinSet,
            time};

use crate::DEVICE_CTX_INSTANCE;

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

        let device_ctx = DEVICE_CTX_INSTANCE.read().await;
        let keyboard_device = if let Some(device_ctx) = device_ctx.as_ref() {
            &device_ctx.keyboard_device
        } else {
            return;
        };
        let keyboard_receiver = Arc::new(Mutex::new(keyboard_device.keyboard_update_sender.subscribe()));
        drop(device_ctx);

        loop {
            let timeout_sender = sender.clone();

            let mut join_set = JoinSet::new();
            join_set.spawn(async move {
                tokio::time::sleep(Duration::from_millis(1000)).await;
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
            if process_message(msg, who).await.is_break() {
                break;
            }
        }
    });

    let _ = join_set.join_next().await;
    join_set.shutdown().await;


    println!("Websocket context {} destroyed", who);
}

async fn send_keyboard_update() -> ControlFlow<(), ()> {
    let mut join_set = JoinSet::new();
    join_set.spawn(async move {
        let device_ctx = DEVICE_CTX_INSTANCE.read().await;
        let keyboard_device = if let Some(device_ctx) = device_ctx.as_ref() {
            &device_ctx.keyboard_device
        } else {
            return ControlFlow::Break(());
        };

        if let Err(err) = keyboard_device.send().await {
            eprintln!("keyboard_device.send failed: {err}");
        }
        if let Err(err) = keyboard_device.send_legacy().await {
            eprintln!("keyboard_device.send_legacy failed: {err}");
        }
        ControlFlow::Continue(())
    });
    join_set.spawn(async {
        time::sleep(Duration::from_secs(5)).await;
        eprintln!("keyboard_device send timeout.");
        ControlFlow::Break(())
    });

    let ret = join_set.join_next().await.unwrap().unwrap();
    join_set.shutdown().await;
    ret
}

async fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<(), ()> {
    match msg {
        Message::Binary(d) => {
            let device_ctx = DEVICE_CTX_INSTANCE.read().await;
            let keyboard_device = if let Some(device_ctx) = device_ctx.as_ref() {
                &device_ctx.keyboard_device
            } else {
                return ControlFlow::Break(());
            };
            if d.len() != 3 || (d[0] != 0 && d[0] != 1) {
                return ControlFlow::Break(());
            } else if d[0] == 0 {
                if d[2] != 0 && d[2] != 1 {
                    return ControlFlow::Break(());
                }
                if keyboard_device.set_key(d[1] as u16, d[2] == 1).await {
                    return send_keyboard_update().await;
                }
            } else {
                if d[2] != 0 || d[2] != 1 {
                    return ControlFlow::Break(());
                }
                if keyboard_device.set_sys_control_key(d[1] as u16, d[2] == 1).await {
                    return send_keyboard_update().await;
                }
            }
            ControlFlow::Continue(())
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
            ControlFlow::Break(())
        }
        _ => {
            ControlFlow::Continue(())
        }
    }
}
