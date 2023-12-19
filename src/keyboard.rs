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
use tokio::{
    sync::{Mutex, RwLock},
    task::JoinSet,
    time,
};

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
    let device_ctx_send = device_ctx.clone();
    join_set.spawn(async move {
        let keyboard_device = &device_ctx_send.read().await.keyboard_device;
        let mut keyboard_receiver = keyboard_device.keyboard_update_sender.subscribe();

        // update at first
        let keyboard_status = keyboard_receiver.borrow_and_update().to_vec();
        if sender.send(Message::Binary(keyboard_status)).await.is_err()
            || sender.flush().await.is_err()
        {
            return;
        }

        let sender = Arc::new(Mutex::new(sender));
        let keyboard_receiver = Arc::new(Mutex::new(keyboard_receiver));

        loop {
            let timeout_sender = sender.clone();

            let mut join_set = JoinSet::new();
            join_set.spawn(async move {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                if timeout_sender
                    .lock()
                    .await
                    .send(Message::Ping(vec![1, 2, 3]))
                    .await
                    .is_ok()
                {
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
                if keyboard_receiver.changed().await.is_ok() {
                    let keyboard_status = keyboard_receiver.borrow_and_update().to_vec();
                    if keyboard_status_sender
                        .send(Message::Binary(keyboard_status))
                        .await
                        .is_ok()
                        && keyboard_status_sender.flush().await.is_ok()
                    {
                        ControlFlow::Continue(())
                    } else {
                        ControlFlow::Break(())
                    }
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

    let device_ctx_recv = device_ctx.clone();
    join_set.spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if process_message(device_ctx_recv.clone(), msg, who)
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

async fn send_keyboard_update(device_ctx: Arc<RwLock<DeviceCtx>>) -> ControlFlow<(), ()> {
    let mut join_set = JoinSet::new();
    join_set.spawn(async move {
        let keyboard_device = &device_ctx.read().await.keyboard_device;

        if let Err(err) = keyboard_device.send().await {
            log::error!("keyboard_device.send failed: {err}");
        }
        if let Err(err) = keyboard_device.send_legacy().await {
            log::error!("keyboard_device.send_legacy failed: {err}");
        }
        ControlFlow::Continue(())
    });
    join_set.spawn(async {
        time::sleep(Duration::from_secs(5)).await;
        log::warn!("keyboard_device send timeout.");
        ControlFlow::Continue(())
    });

    let ret = join_set.join_next().await.unwrap().unwrap();
    join_set.shutdown().await;
    ret
}

async fn process_message(
    device_ctx: Arc<RwLock<DeviceCtx>>,
    msg: Message,
    who: SocketAddr,
) -> ControlFlow<(), ()> {
    match msg {
        Message::Binary(d) => {
            let keyboard_device = &device_ctx.read().await.keyboard_device;
            if d.len() != 3 || (d[0] != 0 && d[0] != 1) {
                return ControlFlow::Break(());
            } else if d[0] == 0 {
                if d[2] != 0 && d[2] != 1 {
                    return ControlFlow::Break(());
                }
                if keyboard_device.set_key(d[1] as u16, d[2] == 1).await {
                    return send_keyboard_update(device_ctx.clone()).await;
                }
            } else {
                if d[2] != 0 || d[2] != 1 {
                    return ControlFlow::Break(());
                }
                if keyboard_device
                    .set_sys_control_key(d[1] as u16, d[2] == 1)
                    .await
                {
                    return send_keyboard_update(device_ctx.clone()).await;
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
        _ => ControlFlow::Continue(()),
    }
}
