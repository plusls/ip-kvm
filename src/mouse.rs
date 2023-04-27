use std::{net::SocketAddr,
          ops::ControlFlow,
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
use tokio::{
    task::JoinSet,
    time};

use usb_otg::hid::mouse::Mouse;

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
    let (mut sender, mut receiver) = socket.split();

    let mut join_set = JoinSet::new();

    join_set.spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            if sender.send(Message::Ping(vec![1, 2, 3])).await.is_err() {
                break;
            }
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

async fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<(), ()> {
    match msg {
        Message::Binary(d) => {
            // 6 byte
            // button -> 1
            // X -> 2
            // Y -> 2
            // wheel -> 1

            if d.len() != 6 {
                return ControlFlow::Break(());
            }
            let mut u16_buf = [0_u8; 2];
            u16_buf.copy_from_slice(&d[1..3]);
            let x = u16::from_le_bytes(u16_buf);
            if x > Mouse::ABS_MAX {
                return ControlFlow::Break(());
            }
            u16_buf.copy_from_slice(&d[3..5]);
            let y = u16::from_le_bytes(u16_buf);
            if y > Mouse::ABS_MAX {
                return ControlFlow::Break(());
            }
            let wheel = d[5] as i8;
            if wheel == -128 {
                return ControlFlow::Break(());
            }
            let device_ctx = DEVICE_CTX_INSTANCE.read().await;
            let mouse_device = if let Some(device_ctx) = device_ctx.as_ref() {
                &device_ctx.mouse_device
            } else {
                return ControlFlow::Break(());
            };

            mouse_device.mouse.lock().await.button = d[0];
            drop(device_ctx);
            let mut join_set = JoinSet::new();
            join_set.spawn(async move {
                let device_ctx = DEVICE_CTX_INSTANCE.read().await;
                let mouse_device = if let Some(device_ctx) = device_ctx.as_ref() {
                    &device_ctx.mouse_device
                } else {
                    return ControlFlow::Break(());
                };
                if let Err(err) = mouse_device.send(x, y, wheel).await {
                    eprintln!("mouse_device.send failed: {err}");
                }
                ControlFlow::Continue(())
            });
            join_set.spawn(async {
                time::sleep(Duration::from_secs(5)).await;
                eprintln!("mouse_device send timeout.");
                ControlFlow::Break(())
            });

            let ret = join_set.join_next().await.unwrap().unwrap();
            join_set.shutdown().await;
            ret
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
