use std::sync::Arc;
use std::{any::Any, net::SocketAddr, path::PathBuf, time::Duration};

use axum::{
    body::Body,
    extract::{Extension, State},
    http::{uri::Uri, Request},
    response::{IntoResponse, Response},
    routing, Router,
};

use hyper::StatusCode;

use hyper_util::{client::legacy::connect::HttpConnector, rt::TokioExecutor};

use tokio::{
    main, signal,
    sync::{Mutex, RwLock},
    task::JoinSet,
    time,
};
use tower_http::{
    services::ServeDir,
    trace::{DefaultMakeSpan, TraceLayer},
};
use util::error;

use usb_otg::{hid, Configurable, GadgetInfo, UsbConfiguration};

mod api_error;
mod keyboard;
mod mass_storage;
mod mouse;
mod mouse_legacy;

const CONFIGFS_BASE: &str = "/sys/kernel/config/usb_gadget";

pub struct DeviceCtx {
    usb_gadget_path: String,
    keyboard_device: hid::keyboard::KeyboardDevice,
    mouse_device: hid::mouse::MouseDevice,
    join_set: Mutex<JoinSet<()>>,
}

const UDC_PATH: &str = "/sys/class/udc";

impl DeviceCtx {
    pub async fn new(configfs_base: &str) -> error::Result<Arc<RwLock<Self>>> {
        let mut gadget_info: GadgetInfo = Default::default();
        gadget_info.functions.insert(
            "hid.usb0".into(),
            Box::new(usb_otg::hid::keyboard::KEYBOARD_LEGACY_FHO.clone()),
        );
        gadget_info.functions.insert(
            "hid.usb1".into(),
            Box::new(usb_otg::hid::keyboard::KEYBOARD_FHO.clone()),
        );
        gadget_info.functions.insert(
            "hid.usb2".into(),
            Box::new(usb_otg::hid::mouse::MOUSE_LEGACY_FHO.clone()),
        );
        gadget_info.functions.insert(
            "hid.usb3".into(),
            Box::new(usb_otg::hid::mouse::MOUSE_FHO.clone()),
        );
        gadget_info.functions.insert(
            "mass_storage.usb0".into(),
            Box::new(usb_otg::mass_storage::FunctionMsgOpts::default()),
        );

        let mut usb_config: UsbConfiguration = Default::default();
        usb_config.strings.insert(0x409, Default::default());
        usb_config.functions.push("hid.usb0".into());
        usb_config.functions.push("hid.usb1".into());
        usb_config.functions.push("hid.usb2".into());
        usb_config.functions.push("hid.usb3".into());
        // usb_config.functions.push("mass_storage.usb0".into());

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

        log::info!("UDC name: {}", gadget_info.udc);

        gadget_info.bcd_usb = 0x210; // USB 2.1

        let usb_gadget_path = format!("{configfs_base}/ip-kvm");
        GadgetInfo::cleanup(&usb_gadget_path)?;
        gadget_info.apply_config(&usb_gadget_path)?;

        let keyboard_legacy_minor = (gadget_info.functions.get("hid.usb0").unwrap().as_ref()
            as &dyn Any)
            .downcast_ref::<hid::FunctionHidOpts>()
            .unwrap()
            .minor;

        let keyboard_minor = (gadget_info.functions.get("hid.usb1").unwrap().as_ref() as &dyn Any)
            .downcast_ref::<hid::FunctionHidOpts>()
            .unwrap()
            .minor;

        let mouse_legacy_minor = (gadget_info.functions.get("hid.usb2").unwrap().as_ref()
            as &dyn Any)
            .downcast_ref::<hid::FunctionHidOpts>()
            .unwrap()
            .minor;

        let mouse_minor = (gadget_info.functions.get("hid.usb3").unwrap().as_ref() as &dyn Any)
            .downcast_ref::<hid::FunctionHidOpts>()
            .unwrap()
            .minor;

        log::info!(
            "keyboard_legacy_minor: {keyboard_legacy_minor} keyboard_minor: {keyboard_minor}"
        );
        log::info!("mouse_legacy_minor: {mouse_legacy_minor} mouse_minor: {mouse_minor}");

        let hid_path_list: Vec<_> = [
            keyboard_legacy_minor,
            keyboard_minor,
            mouse_legacy_minor,
            mouse_minor,
        ]
        .iter()
        .map(|hid_id| std::path::Path::new(&format!("/dev/hidg{hid_id}")).to_path_buf())
        .collect();

        // 等待 hidg 设备创建完毕
        while hid_path_list.iter().any(|hid_path| !hid_path.exists()) {
            time::sleep(Duration::from_millis(500)).await;
        }

        let keyboard_device =
            hid::keyboard::KeyboardDevice::new(keyboard_minor, keyboard_legacy_minor).await?;
        let mouse_device = hid::mouse::MouseDevice::new(mouse_minor, mouse_legacy_minor).await?;
        let ret = Arc::new(RwLock::new(Self {
            join_set: Mutex::new(JoinSet::new()),
            keyboard_device,
            mouse_device,
            usb_gadget_path,
        }));
        let device_ctx = ret.write().await;
        let join_set = &device_ctx.join_set;
        let ret_recv = ret.clone();
        join_set.lock().await.spawn(async move {
            let device_ctx = ret_recv.read().await;
            let keyboard_device = &device_ctx.keyboard_device;
            loop {
                keyboard_device.recv().await.unwrap();
            }
        });
        let recv_legacy = ret.clone();
        join_set.lock().await.spawn(async move {
            let device_ctx = recv_legacy.read().await;
            let keyboard_device = &device_ctx.keyboard_device;
            loop {
                keyboard_device.recv_legacy().await.unwrap();
            }
        });
        drop(device_ctx);
        Ok(ret)
    }
    pub async fn abort_join_set(&self) {
        log::info!("DeviceCtx start shutdown.");
        self.join_set.lock().await.shutdown().await;
        log::info!("DeviceCtx shutdown success.");
    }
}

impl Drop for DeviceCtx {
    fn drop(&mut self) {
        if let Err(err) = GadgetInfo::cleanup(&self.usb_gadget_path) {
            log::error!("GadgetInfo cleanup failed: {err}");
        } else {
            log::info!("GadgetInfo cleanup success.");
        }
    }
}

type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;

#[main]
async fn main() -> error::Result<()> {
    let mut join_set = JoinSet::new();

    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    // TODO handle
    let device_ctx = DeviceCtx::new(CONFIGFS_BASE).await.unwrap();
    let device_ctx_recv = device_ctx.clone();
    join_set.spawn(async move {
        let device_ctx_recv = device_ctx_recv.read().await;
        let mut join_set = device_ctx_recv.join_set.lock().await;
        let _ = join_set.join_next().await;
    });

    let assets_dir = PathBuf::from("ip-kvm-assets");
    let client: Client =
        hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
            .build(HttpConnector::new());

    let app = Router::new()
        .fallback_service(ServeDir::new(assets_dir).append_index_html_on_directories(true))
        .route("/stream", routing::get(stream_handler))
        .route("/v1/ws/keyboard", routing::get(keyboard::ws_handler))
        .route("/v1/ws/mouse", routing::get(mouse::ws_handler))
        .route(
            "/v1/ws/mouse_legacy",
            routing::get(mouse_legacy::ws_handler),
        )
        .route("/v1/usb-images", routing::get(mass_storage::get_images))
        .route(
            "/v1/usb-image/:file_name",
            routing::get(mass_storage::get_image).delete(mass_storage::delete_image),
        )
        .route(
            "/v1/usb-image/:file_name/block/:offset",
            routing::put(mass_storage::put_image_block),
        )
        .route(
            "/v1/current-image",
            routing::put(mass_storage::put_current_image),
        )
        .with_state(client)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(Extension(device_ctx.clone()));

    let addr = "0.0.0.0:3000";

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    log::info!("listening on {}", listener.local_addr().unwrap());

    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    );

    join_set.spawn(async { server.await.unwrap() });

    join_set.spawn(async {
        match signal::ctrl_c().await {
            Ok(()) => {}
            Err(err) => {
                log::error!("Unable to listen for shutdown signal: {}", err);
            }
        }
    });

    let _ = join_set.join_next().await;

    log::info!("Now shutdown IP-KVM...");
    join_set.shutdown().await;
    device_ctx.read().await.abort_join_set().await;
    log::info!("IP-KVM join_set shutdown.");

    Ok(())
}

async fn stream_handler(
    State(client): State<Client>,
    mut req: Request<Body>,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    // TODO 可配置的 stream URL
    let uri = format!("http://127.0.0.1:3001{}", path_query);

    *req.uri_mut() = Uri::try_from(uri).unwrap();

    Ok(client
        .request(req)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .into_response())
}
