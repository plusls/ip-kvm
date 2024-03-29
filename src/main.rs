use std::{any::Any, net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};

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

use clap::Parser;

use usb_otg::{hid, Configurable, GadgetInfo, UsbConfiguration};
use util::error;

mod api_error;
mod keyboard;
mod mass_storage;
mod mouse;
mod mouse_legacy;

const CONFIGFS_BASE: &str = "/sys/kernel/config/usb_gadget";

pub struct DeviceCtx {
    usb_gadget_path: String,
    hid_composite_device: hid::hid_composite::HidCompositeDevice,
    keyboard_device: hid::keyboard::KeyboardDevice,
    mouse_device: hid::mouse::MouseDevice,
    join_set: Mutex<JoinSet<()>>,
}

const UDC_PATH: &str = "/sys/class/udc";
const CONFIGURE_NAME: &str = "c.1";
const FUNCTION_NAME_KEYBOARD_LEGACY: &str = "hid.keyboard_legacy";
const FUNCTION_NAME_MOUSE_LEGACY: &str = "hid.mouse_legacy";
const FUNCTION_NAME_HID_COMPOSITE: &str = "hid.hid_composite";
const FUNCTION_NAME_MSG: &str = "mass_storage.msg";
const LUN_COUNT: u8 = 8;
impl DeviceCtx {
    pub async fn new(configfs_base: &str) -> error::Result<Arc<RwLock<Self>>> {
        let mut gadget_info: GadgetInfo = Default::default();
        gadget_info.functions.insert(
            FUNCTION_NAME_KEYBOARD_LEGACY.into(),
            Box::new(hid::keyboard::KEYBOARD_LEGACY_FHO.clone()),
        );
        gadget_info.functions.insert(
            FUNCTION_NAME_MOUSE_LEGACY.into(),
            Box::new(hid::mouse::MOUSE_LEGACY_FHO.clone()),
        );
        gadget_info.functions.insert(
            FUNCTION_NAME_HID_COMPOSITE.into(),
            Box::new(hid::hid_composite::HID_COMPOSITE_FHO.clone()),
        );

        let mut function_msg_opt = usb_otg::mass_storage::FunctionMsgOpts::default();

        for i in 1..LUN_COUNT {
            function_msg_opt.luns.insert(
                usb_otg::mass_storage::FunctionMsgOpts::lun_name(i as u8),
                usb_otg::mass_storage::MsgLun::default(),
            );
        }

        for entry in function_msg_opt.luns.iter_mut() {
            entry.1.removable = true;
        }

        gadget_info
            .functions
            .insert(FUNCTION_NAME_MSG.into(), Box::new(function_msg_opt));

        let mut usb_config: UsbConfiguration = Default::default();
        usb_config
            .strings
            .insert(usb_otg::LANGUAGE_CODE_ENGLISH, Default::default());
        for function_name in gadget_info.functions.keys() {
            usb_config.functions.push(function_name.into());
        }

        gadget_info
            .configs
            .insert(CONFIGURE_NAME.into(), usb_config);
        gadget_info
            .strings
            .insert(usb_otg::LANGUAGE_CODE_ENGLISH, Default::default());

        let mut udc_name = None;
        for entry in util::fs::read_dir(UDC_PATH)? {
            let entry = entry.map_err(|err| error::ErrorKind::io(err, UDC_PATH))?;
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

        let keyboard_legacy_minor = (gadget_info
            .functions
            .get(FUNCTION_NAME_KEYBOARD_LEGACY)
            .unwrap()
            .as_ref() as &dyn Any)
            .downcast_ref::<hid::FunctionHidOpts>()
            .unwrap()
            .minor;

        let mouse_legacy_minor = (gadget_info
            .functions
            .get(FUNCTION_NAME_MOUSE_LEGACY)
            .unwrap()
            .as_ref() as &dyn Any)
            .downcast_ref::<hid::FunctionHidOpts>()
            .unwrap()
            .minor;

        let hid_composite_minor = (gadget_info
            .functions
            .get(FUNCTION_NAME_HID_COMPOSITE)
            .unwrap()
            .as_ref() as &dyn Any)
            .downcast_ref::<hid::FunctionHidOpts>()
            .unwrap()
            .minor;

        log::info!(
            "keyboard_legacy_minor: {keyboard_legacy_minor} mouse_legacy_minor: {mouse_legacy_minor} hid_composite_minor: {hid_composite_minor}"
        );

        let hid_path_list: Vec<_> = [
            keyboard_legacy_minor,
            mouse_legacy_minor,
            hid_composite_minor,
        ]
        .iter()
        .map(|hid_id| std::path::Path::new(&format!("/dev/hidg{hid_id}")).to_path_buf())
        .collect();

        // 等待 hidg 设备创建完毕
        while hid_path_list.iter().any(|hid_path| !hid_path.exists()) {
            time::sleep(Duration::from_millis(500)).await;
        }
        let hid_composite_device =
            hid::hid_composite::HidCompositeDevice::new(hid_composite_minor).await?;
        let keyboard_device = hid::keyboard::KeyboardDevice::new(
            keyboard_legacy_minor,
            hid_composite_device.hid_composite_dev_send_sender.clone(),
        )
        .await?;
        let mouse_device = hid::mouse::MouseDevice::new(
            mouse_legacy_minor,
            hid_composite_device.hid_composite_dev_send_sender.clone(),
        )
        .await?;
        let ret = Arc::new(RwLock::new(Self {
            join_set: Mutex::new(JoinSet::new()),
            hid_composite_device,
            keyboard_device,
            mouse_device,
            usb_gadget_path,
        }));
        let device_ctx = ret.write().await;
        let join_set = &device_ctx.join_set;
        let ret_recv = ret.clone();

        // 从复合 hid 设备中读取响应，并根据类型转发给对应的设备
        join_set.lock().await.spawn(async move {
            let device_ctx = ret_recv.read().await;
            let hid_composite_device = &device_ctx.hid_composite_device;
            let keyboard_device = &device_ctx.keyboard_device;
            loop {
                let res = hid_composite_device.recv().await;

                if let Err(error::Error(err)) = &res {
                    if let error::ErrorKind::Ignore = err.as_ref() {
                        continue;
                    }
                }
                let payload = res.unwrap();
                assert!(payload[0] == hid::hid_composite::HID_REPORT_ID_KEYBOARD);
                keyboard_device.recv(&payload).await.unwrap();
            }
        });

        // 复合 hid 设备接受来自其它设备的请求，并发送给 /dev/
        let hid_composite_ret = ret.clone();
        join_set.lock().await.spawn(async move {
            let device_ctx = hid_composite_ret.read().await;
            let hid_composite_device = &device_ctx.hid_composite_device;
            let mut hid_composite_dev_receiver = hid_composite_device
                .hid_composite_dev_send_sender
                .subscribe();

            loop {
                if hid_composite_dev_receiver.changed().await.is_ok() {
                    let hid_composite_send_data =
                        hid_composite_dev_receiver.borrow_and_update().to_vec();
                    // 电脑关机后 send 会失败
                    let _ = hid_composite_device.send(&hid_composite_send_data).await;
                }
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
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, default_value = "127.0.0.1:3000")]
    server_listen_addr: String,
    #[arg(long, default_value = "127.0.0.1:3001")]
    vnc_listen_addr: String,
    #[arg(long, default_value = "http://127.0.0.1:3002")]
    ustreamer_url: String,
    #[arg(long, default_value = "images")]
    image_dir: String,
}

type Client = hyper_util::client::legacy::Client<HttpConnector, Body>;

struct AppState {
    args: Args,
    http_client: Client,
}

#[main]
async fn main() -> error::Result<()> {
    let http_client: Client =
        hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
            .build(HttpConnector::new());

    pretty_env_logger::formatted_builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let mut join_set = JoinSet::new();

    let device_ctx = DeviceCtx::new(CONFIGFS_BASE).await?;
    let device_ctx_recv = device_ctx.clone();
    join_set.spawn(async move {
        let device_ctx_recv = device_ctx_recv.read().await;
        let mut join_set = device_ctx_recv.join_set.lock().await;
        let _ = join_set.join_next().await;
    });

    let assets_dir = PathBuf::from("ip-kvm-assets");

    let app_state = Arc::new(AppState {
        args: Args::parse(),
        http_client,
    });

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
        .with_state(app_state.clone())
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(Extension(device_ctx.clone()));

    let listener = tokio::net::TcpListener::bind(&app_state.args.server_listen_addr)
        .await
        .map_err(|err| error::ErrorKind::io(err, &app_state.args.server_listen_addr))?;

    log::info!(
        "listening on {}",
        listener
            .local_addr()
            .map_err(|err| error::ErrorKind::io(err, &app_state.args.server_listen_addr))?
    );

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
    State(app_state): State<Arc<AppState>>,
    mut req: Request<Body>,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    let uri = format!("{}{path_query}", app_state.args.ustreamer_url);

    *req.uri_mut() = Uri::try_from(uri).unwrap();

    Ok(app_state
        .http_client
        .request(req)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
        .into_response())
}
