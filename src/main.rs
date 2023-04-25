#![feature(trait_upcasting)]

use std::any::Any;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    Router,
    routing,
    Server,
};
use axum::extract::State;
use axum::http::Request;
use axum::http::uri::Uri;
use axum::response::Response;
use hyper::{Body, client::HttpConnector};
use once_cell::sync::OnceCell;
use tokio::task::JoinSet;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use util::error;
use util::error::ErrorKind;

use usb_otg::{Configurable, GadgetInfo, hid, UsbConfiguration};

mod keyboard;
mod mouse;
mod mouse_legacy;

const CONFIGFS_BASE: &str = "/sys/kernel/config/usb_gadget";


struct DeviceCtx {
    configfs_base: String,
    keyboard_device: hid::keyboard::KeyboardDevice,
    mouse_device: hid::mouse::MouseDevice,
}

impl Drop for DeviceCtx {
    fn drop(&mut self) {
        GadgetInfo::cleanup(&self.configfs_base).unwrap();
    }
}


static DEVICE_CTX_INSTANCE: OnceCell<Arc<DeviceCtx>> = OnceCell::new();

const UDC_PATH: &str = "/sys/class/udc";

impl DeviceCtx {
    pub fn instance() -> Arc<Self> {
        // 肯定不会是空，必然是在 DEVICE_CTX 初始化后才走到这
        return DEVICE_CTX_INSTANCE.get().unwrap().clone();
    }

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
            let entry = entry.map_err(|err| ErrorKind::fs(err, UDC_PATH))?;
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
            Err(ErrorKind::custom("Can not found udc".into()))?;
        }


        gadget_info.bcd_usb = 0x210;  // USB 2.1

        let usb_gadget_path = format!("{CONFIGFS_BASE}/ip-kvm");
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

        let keyboard_device = hid::keyboard::KeyboardDevice::new(keyboard_minor, keyboard_legacy_minor).await?;
        let mouse_device = hid::mouse::MouseDevice::new(mouse_minor, mouse_legacy_minor).await?;
        DEVICE_CTX_INSTANCE.set(Arc::new(Self {
            configfs_base: configfs_base.into(),
            keyboard_device,
            mouse_device,
        })).unwrap_or(());

        Ok(())
    }
}

#[tokio::main]
async fn main() -> error::Result<()> {
    DeviceCtx::init(CONFIGFS_BASE).await?;
    let mut join_set = JoinSet::new();
    {
        let keyboard_device = &DEVICE_CTX_INSTANCE.get().unwrap().keyboard_device;
        join_set.spawn(keyboard_device.recv_loop());
        join_set.spawn(keyboard_device.recv_legacy_loop());
    }

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
    // .serve(app.into_make_service());

    join_set.spawn(async { server.await.unwrap() });

    while let Some(_) = join_set.join_next().await {}
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
