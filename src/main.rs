#![feature(trait_upcasting)]

use std::{any::Any,
          net::SocketAddr,
          path::PathBuf,
          time::Duration,
};

use axum::{
    extract::State,
    http::{Request, uri::Uri},
    response::Response,
    Router,
    routing,
    Server,
};
use hyper::{Body, client::HttpConnector};
use once_cell::sync::Lazy;
use tokio::{main,
            signal,
            sync::RwLock,
            task::JoinSet,
            time,
};
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use util::error;

use usb_otg::{Configurable, GadgetInfo, hid, UsbConfiguration};

mod keyboard;
mod mouse;
mod mouse_legacy;

const CONFIGFS_BASE: &str = "/sys/kernel/config/usb_gadget";


pub struct DeviceCtx {
    keyboard_device: hid::keyboard::KeyboardDevice,
    mouse_device: hid::mouse::MouseDevice,
}


pub static DEVICE_CTX_INSTANCE: Lazy<RwLock<Option<DeviceCtx>>> = Lazy::new(|| RwLock::new(None));

const UDC_PATH: &str = "/sys/class/udc";

impl DeviceCtx {
    pub async fn init(configfs_base: &str) -> error::Result<()> {
        let mut gadget_info: GadgetInfo = Default::default();
        gadget_info.functions.insert("hid.usb0".into(), Box::new(usb_otg::hid::keyboard::KEYBOARD_LEGACY_FHO.clone()));
        gadget_info.functions.insert("hid.usb1".into(), Box::new(usb_otg::hid::keyboard::KEYBOARD_FHO.clone()));
        gadget_info.functions.insert("hid.usb2".into(), Box::new(usb_otg::hid::mouse::MOUSE_LEGACY_FHO.clone()));
        gadget_info.functions.insert("hid.usb3".into(), Box::new(usb_otg::hid::mouse::MOUSE_FHO.clone()));

        let mut usb_config: UsbConfiguration = Default::default();
        usb_config.strings.insert(0x409, Default::default());
        usb_config.functions.push("hid.usb0".into());
        usb_config.functions.push("hid.usb1".into());
        usb_config.functions.push("hid.usb2".into());
        usb_config.functions.push("hid.usb3".into());

        gadget_info.configs.insert("c.1".into(), usb_config);
        gadget_info.strings.insert(0x409, Default::default());


        let mut udc_name = None;
        for entry in util::fs::read_dir(UDC_PATH)? {
            let entry = entry.map_err(|err| error::ErrorKind::fs(err, UDC_PATH))?;
            let path = entry.path();
            if let Some(path_file_name) = path.file_name() {
                if let Some(path_file_name) = path_file_name.to_str() {
                    udc_name = Some(path_file_name.to_string());
                    break;
                }
            }
        }

        if let Some(udc_name) = udc_name {
            gadget_info.udc = udc_name;
        } else {
            Err(error::ErrorKind::custom("Can not found udc".into()))?;
        }


        gadget_info.bcd_usb = 0x210;  // USB 2.1

        let usb_gadget_path = format!("{configfs_base}/ip-kvm");
        GadgetInfo::cleanup(&usb_gadget_path)?;
        gadget_info.apply_config(&usb_gadget_path)?;

        let keyboard_legacy_minor = (gadget_info.functions.get("hid.usb0").unwrap().as_ref() as &dyn Any)
            .downcast_ref::<hid::FunctionHidOpts>().unwrap().minor;

        let keyboard_minor = (gadget_info.functions.get("hid.usb1").unwrap().as_ref() as &dyn Any)
            .downcast_ref::<hid::FunctionHidOpts>().unwrap().minor;

        let mouse_legacy_minor = (gadget_info.functions.get("hid.usb2").unwrap().as_ref() as &dyn Any)
            .downcast_ref::<hid::FunctionHidOpts>().unwrap().minor;

        let mouse_minor = (gadget_info.functions.get("hid.usb3").unwrap().as_ref() as &dyn Any)
            .downcast_ref::<hid::FunctionHidOpts>().unwrap().minor;

        println!("keyboard_legacy_minor: {keyboard_legacy_minor} keyboard_minor: {keyboard_minor}");
        println!("mouse_legacy_minor: {mouse_legacy_minor} mouse_minor: {mouse_minor}");

        let hid_path_list: Vec<_> =
            [keyboard_legacy_minor, keyboard_minor, mouse_legacy_minor, mouse_minor].iter()
                .map(|hid_id| std::path::Path::new(&format!("/dev/hidg{hid_id}")).to_path_buf()).collect();

        // 等待 hidg 设备创建完毕
        while hid_path_list.iter().any(|hid_path| !hid_path.exists()) {
            time::sleep(Duration::from_millis(500)).await;
        }


        let keyboard_device = hid::keyboard::KeyboardDevice::new(keyboard_minor, keyboard_legacy_minor).await?;
        let mouse_device = hid::mouse::MouseDevice::new(mouse_minor, mouse_legacy_minor).await?;
        *DEVICE_CTX_INSTANCE.write().await = Some(Self {
            keyboard_device,
            mouse_device,
        });

        Ok(())
    }
}

#[main]
async fn main() -> error::Result<()> {
    let mut join_set = JoinSet::new();

    DeviceCtx::init(CONFIGFS_BASE).await.unwrap();

    join_set.spawn(async {
        let device_ctx = DEVICE_CTX_INSTANCE.read().await;
        let keyboard_device = &device_ctx.as_ref().unwrap().keyboard_device;
        loop {
            keyboard_device.recv().await.unwrap();
        }
    });

    join_set.spawn(async {
        let device_ctx = DEVICE_CTX_INSTANCE.read().await;
        let keyboard_device = &device_ctx.as_ref().unwrap().keyboard_device;
        loop {
            keyboard_device.recv_legacy().await.unwrap();
        }
    });

    let assets_dir = PathBuf::from("ip-kvm-assets");
    let client = Client::new();

    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/stream", routing::get(stream_handler))
        .route("/keyboard", routing::get(keyboard::ws_handler))
        .route("/mouse", routing::get(mouse::ws_handler))
        .route("/mouse_legacy", routing::get(mouse_legacy::ws_handler))

        .with_state(client)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let server = Server::bind(&addr)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>());

    join_set.spawn(async { server.await.unwrap() });


    match signal::ctrl_c().await {
        Ok(()) => {}
        Err(err) => {
            eprintln!("Unable to listen for shutdown signal: {}", err);
        }
    }
    println!("Now shutdown ip-kvm...");

    join_set.shutdown().await;

    println!("Wait DEVICE_CTX_INSTANCE write lock.");
    *DEVICE_CTX_INSTANCE.write().await = None;
    println!("Drop device ctx success!");

    GadgetInfo::cleanup(&format!("{CONFIGFS_BASE}/ip-kvm"))?;
    println!("GadgetInfo cleanup success.");
    Ok(())
}

type Client = hyper::client::Client<HttpConnector, Body>;

async fn stream_handler(State(client): State<Client>, mut req: Request<Body>) -> Response<Body> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    // TODO 可配置的 stream URL
    let uri = format!("http://127.0.0.1:3001{}", path_query);

    *req.uri_mut() = Uri::try_from(uri).unwrap();

    client.request(req).await.unwrap()
}

