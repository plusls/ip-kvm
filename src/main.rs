#![feature(trait_upcasting)]

use std::any::Any;
use std::net::SocketAddr;
use std::ops::ControlFlow;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{
        TypedHeader,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    headers,
    response::IntoResponse,
    Router,
    routing,
    Server,
};
use axum::extract::connect_info::ConnectInfo;
use futures::{sink::SinkExt, stream::StreamExt};
use once_cell::sync::OnceCell;
use tokio::sync::Mutex;
use tokio::task::JoinSet;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use util::error;

use usb_otg::{Configurable, GadgetInfo, hid, UsbConfiguration};

const CONFIGFS_BASE: &str = "/sys/kernel/config/usb_gadget";

static DEVICE_CTX: OnceCell<Arc<DeviceCtx>> = OnceCell::new();

struct DeviceCtx {
    configfs_base: String,
    keyboard_device: hid::keyboard::KeyboardDevice,
}

impl Drop for DeviceCtx {
    fn drop(&mut self) {
        GadgetInfo::cleanup(&self.configfs_base).unwrap();
    }
}

impl DeviceCtx {
    async fn new(configfs_base: &str) -> error::Result<Self> {
        let mut gadget_info: GadgetInfo = Default::default();
        gadget_info.functions.insert("hid.usb0".into(), Box::new(usb_otg::hid::keyboard::KEYBOARD_LEGACY_FHO.clone()));
        gadget_info.functions.insert("hid.usb1".into(), Box::new(usb_otg::hid::keyboard::KEYBOARD_FHO.clone()));

        let mut usb_config: UsbConfiguration = Default::default();
        usb_config.strings.insert(0x409, Default::default());
        usb_config.functions.push("hid.usb0".into());
        usb_config.functions.push("hid.usb1".into());

        gadget_info.configs.insert("c.1".into(), usb_config);
        gadget_info.strings.insert(0x409, Default::default());

        gadget_info.udc = "musb-hdrc.5.auto".into();

        gadget_info.bcd_usb = 0x210;  // USB 2.1

        let usb_gadget_path = format!("{CONFIGFS_BASE}/ip-kvm");
        GadgetInfo::cleanup(&usb_gadget_path)?;
        gadget_info.apply_config(&usb_gadget_path)?;

        let keyboard_legacy_minor = (gadget_info.functions.get("hid.usb0").unwrap().as_ref() as &dyn Any)
            .downcast_ref::<hid::FunctionHidOpts>().unwrap().minor;

        let keyboard_minor = (gadget_info.functions.get("hid.usb1").unwrap().as_ref() as &dyn Any)
            .downcast_ref::<hid::FunctionHidOpts>().unwrap().minor;

        println!("keyboard_legacy_minor: {keyboard_legacy_minor} keyboard_minor: {keyboard_minor}");

        let keyboard_device = hid::keyboard::KeyboardDevice::new(keyboard_minor, keyboard_legacy_minor).await?;
        Ok(Self {
            configfs_base: configfs_base.into(),
            keyboard_device,
        })
    }
}

#[tokio::main]
async fn main() -> error::Result<()> {
    let mut join_set = JoinSet::new();
    DEVICE_CTX.set(Arc::new(DeviceCtx::new(CONFIGFS_BASE).await?)).unwrap_or(());
    {
        let keyboard_device = &DEVICE_CTX.get().unwrap().keyboard_device;
        join_set.spawn(keyboard_device.recv_loop());
        join_set.spawn(keyboard_device.recv_legacy_loop());
    }

    let assets_dir = PathBuf::from("ip-kvm-assets");

    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/keyboard", routing::get(ws_handler))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));

    let server = Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>());

    join_set.spawn(async { server.await.unwrap() });

    while let Some(_) = join_set.join_next().await {}
    Ok(())
}

async fn ws_handler(
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

/// helper to print contents of messages to stdout. Has special treatment for Close.
fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<(), ()> {
    match msg {
        Message::Text(t) => {
            println!(">>> {} sent str: {:?}", who, t);
        }
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

        Message::Pong(v) => {
            println!(">>> {} sent pong with {:?}", who, v);
        }
        // You should never need to manually handle Message::Ping, as axum's websocket library
        // will do so for you automagically by replying with Pong and copying the v according to
        // spec. But if you need the contents of the pings you can see them here.
        Message::Ping(v) => {
            println!(">>> {} sent ping with {:?}", who, v);
        }
    }
    ControlFlow::Continue(())
}
