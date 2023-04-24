#![feature(trait_upcasting)]

use std::any::Any;
use std::time::Duration;

use tokio::task::JoinSet;
use util::error;

use usb_otg::{Configurable, GadgetInfo, hid, UsbConfiguration};

const CONFIGFS_BASE: &str = "/sys/kernel/config/usb_gadget";


#[tokio::main]
async fn main() -> error::Result<()> {
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
    println!("start cleanup!");
    GadgetInfo::cleanup(&usb_gadget_path)?;
    println!("start apply_config!");
    gadget_info.apply_config(&usb_gadget_path)?;
    // wait usb device apply
    tokio::time::sleep(Duration::from_secs(2)).await;

    let keyboard_legacy_minor = (gadget_info.functions.get("hid.usb0").unwrap().as_ref() as &dyn Any)
        .downcast_ref::<hid::FunctionHidOpts>().unwrap().minor;

    let keyboard_minor = (gadget_info.functions.get("hid.usb1").unwrap().as_ref() as &dyn Any)
        .downcast_ref::<hid::FunctionHidOpts>().unwrap().minor;

    println!("{keyboard_legacy_minor} {keyboard_minor}");


    let mut join_set = JoinSet::new();

    let keyboard_device = hid::keyboard::KeyboardDevice::new(&mut join_set, keyboard_minor, keyboard_legacy_minor).await?;
    keyboard_device.set_key(hid::keyboard::usage_id::KEYBOARD_UP_ARROW, true).await;

    keyboard_device.send().await;
    keyboard_device.send_legacy().await;

    keyboard_device.set_key(hid::keyboard::usage_id::KEYBOARD_UP_ARROW, false).await;
    keyboard_device.send().await;
    keyboard_device.send_legacy().await;

    while let Some(_) = join_set.join_next().await {}
    Ok(())
}
