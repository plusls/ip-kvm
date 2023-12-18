use std::{net::SocketAddr, ops::ControlFlow, sync::Arc, time::Duration};

use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, WebSocketUpgrade,
    },
    response::IntoResponse,
    Extension,
};

use axum_extra::{headers, TypedHeader};

use futures::{SinkExt, StreamExt};
use tokio::{sync::RwLock, task::JoinSet, time};

use usb_otg::hid::mouse::Mouse;

use crate::DeviceCtx;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Extension(device_ctx): Extension<Arc<RwLock<DeviceCtx>>>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };
    println!("`{user_agent}` at {addr} connected.");
    ws.on_upgrade(move |socket| handle_socket(device_ctx, socket, addr))
}

async fn handle_socket(device_ctx: Arc<RwLock<DeviceCtx>>, socket: WebSocket, who: SocketAddr) {
    let (mut sender, mut receiver) = socket.split();

    let mut join_set = JoinSet::new();

    join_set.spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(1000)).await;
            if sender.send(Message::Ping(vec![1, 2, 3])).await.is_err() {
                break;
            }
        }
    });

    join_set.spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if process_message(device_ctx.clone(), msg, who)
                .await
                .is_break()
            {
                break;
            }
        }
    });

    let _ = join_set.join_next().await;
    join_set.shutdown().await;

    println!("Websocket context {} destroyed", who);
}

async fn process_message(
    device_ctx: Arc<RwLock<DeviceCtx>>,
    msg: Message,
    who: SocketAddr,
) -> ControlFlow<(), ()> {
    match msg {
        Message::Binary(d) => {
            // 4 byte
            // button -> 1
            // X -> 1
            // Y -> 1
            // wheel -> 1

            if d.len() != 4 {
                return ControlFlow::Break(());
            }
            let x = d[1] as i8;
            let y = d[2] as i8;
            let wheel = d[3] as i8;
            if x < Mouse::REL_MIN || y < Mouse::REL_MIN || wheel < Mouse::WHEEL_MIN {
                return ControlFlow::Break(());
            }
            let mouse_device = &device_ctx.read().await.mouse_device;
            mouse_device.mouse.lock().await.button = d[0];
            let mut join_set = JoinSet::new();
            let device_ctx_send = device_ctx.clone();
            join_set.spawn(async move {
                let mouse_device = &device_ctx_send.read().await.mouse_device;
                if let Err(err) = mouse_device.send_legacy(x, y, wheel).await {
                    eprintln!("mouse_legacy_device.send failed: {err}");
                }
                ControlFlow::Continue(())
            });
            join_set.spawn(async {
                time::sleep(Duration::from_secs(5)).await;
                eprintln!("mouse_legacy_device send timeout.");
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
        _ => ControlFlow::Continue(()),
    }
}
