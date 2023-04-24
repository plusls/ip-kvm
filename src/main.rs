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
use once_cell::sync::OnceCell;
use tokio::task::JoinSet;
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use util::error;

use usb_otg::{Configurable, GadgetInfo, hid, UsbConfiguration};

mod keyboard;

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
        .route("/keyboard", routing::get(keyboard::ws_handler))
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
