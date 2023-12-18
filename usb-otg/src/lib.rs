use std::any::Any;
use std::collections::HashMap;
use std::os::unix::fs::symlink;
use std::path::Path;

use util::{error, fs};

use crate::hid::FunctionHidOpts;
use crate::mass_storage::FunctionMsgOpts;

pub mod async_fd;
pub mod hid;
pub mod mass_storage;

pub enum UsbDeviceSpeed {
    // enumerating
    UsbSpeedUnknown,
    // usb 1.1
    UsbSpeedLow,
    UsbSpeedFull,
    // usb 2.0
    UsbSpeedHigh,
    // wireless (usb 2.5)
    UsbSpeedWireless,
    // usb 3.0
    UsbSpeedSuper,
    // usb 3.1
    UsbSpeedSuperPlus,
}

impl UsbDeviceSpeed {
    fn as_str(&self) -> &'static str {
        match self {
            Self::UsbSpeedUnknown => "UNKNOWN",
            Self::UsbSpeedLow => "low-speed",
            Self::UsbSpeedFull => "full-speed",
            Self::UsbSpeedHigh => "high-speed",
            Self::UsbSpeedWireless => "wireless",
            Self::UsbSpeedSuper => "super-speed",
            Self::UsbSpeedSuperPlus => "super-speed-plus",
        }
    }
}

pub trait Configurable: Any {
    fn apply_config(&mut self, base_dir: &dyn AsRef<Path>) -> error::Result<()>;
    fn from_config(_base_dir: &dyn AsRef<Path>) -> error::Result<Self>
    where
        Self: Sized,
    {
        unimplemented!()
    }
    fn cleanup<P: AsRef<Path>>(base_dir: P) -> error::Result<()>
    where
        Self: Sized,
    {
        let base_dir = base_dir.as_ref();
        if !base_dir.is_dir() {
            return Ok(());
        }
        fs::remove_dir(base_dir)?;
        Ok(())
    }
}

pub trait UsbFunctionOpts: Configurable {}

struct FunctionDummyOpts {}

impl Configurable for FunctionDummyOpts {
    fn apply_config(&mut self, _base_dir: &dyn AsRef<Path>) -> error::Result<()> {
        unreachable!()
    }
}

impl UsbFunctionOpts for FunctionDummyOpts {}

pub struct GadgetInfo {
    pub bcd_device: u16,
    pub bcd_usb: u16,
    pub b_device_class: u8,
    pub b_device_protocol: u8,
    pub b_device_sub_class: u8,
    pub b_max_packet_size0: u8,
    pub configs: HashMap<String, UsbConfiguration>,
    pub functions: HashMap<String, Box<dyn UsbFunctionOpts + Sync + Send>>,
    pub id_product: u16,
    pub id_vendor: u16,
    pub max_speed: UsbDeviceSpeed,
    pub os_desc: Option<OsDesc>,
    pub strings: HashMap<u16, GadgetStrings>,
    pub udc: String,
}

impl Default for GadgetInfo {
    fn default() -> Self {
        Self {
            bcd_device: 0x515,
            bcd_usb: 0,
            b_device_class: 0,
            b_device_protocol: 0,
            b_device_sub_class: 0,
            b_max_packet_size0: 0,
            configs: HashMap::new(),
            functions: HashMap::new(),
            id_product: 0,
            id_vendor: 0,
            max_speed: UsbDeviceSpeed::UsbSpeedSuperPlus,
            os_desc: None,
            strings: HashMap::new(),
            udc: "\n".to_string(),
        }
    }
}

impl Configurable for GadgetInfo {
    fn apply_config(&mut self, base_dir: &dyn AsRef<Path>) -> error::Result<()> {
        let base_dir = base_dir.as_ref();
        fs::create_dir(base_dir)?;
        fs::write(base_dir.join("bcdDevice"), self.bcd_device.to_string())?;
        fs::write(base_dir.join("bcdUSB"), self.bcd_usb.to_string())?;
        fs::write(
            base_dir.join("bDeviceClass"),
            self.b_device_class.to_string(),
        )?;
        fs::write(
            base_dir.join("bDeviceProtocol"),
            self.b_device_protocol.to_string(),
        )?;
        fs::write(
            base_dir.join("bDeviceSubClass"),
            self.b_device_sub_class.to_string(),
        )?;
        fs::write(
            base_dir.join("bMaxPacketSize0"),
            self.b_max_packet_size0.to_string(),
        )?;
        let functions_base_dir = base_dir.join("functions");
        for entry in &mut self.functions {
            entry.1.apply_config(&functions_base_dir.join(entry.0))?;
        }
        let configs_base_dir = base_dir.join("configs");
        for entry in &mut self.configs {
            entry.1.apply_config(&configs_base_dir.join(entry.0))?;
        }
        fs::write(base_dir.join("idProduct"), self.id_product.to_string())?;
        fs::write(base_dir.join("idVendor"), self.id_vendor.to_string())?;
        // 低版本内核可能没有这个
        let _ = fs::write(base_dir.join("max_speed"), self.max_speed.as_str());
        if let Some(os_desc) = &mut self.os_desc {
            os_desc.apply_config(&base_dir.join("os_desc"))?;
        }
        let strings_base_dir = base_dir.join("strings");
        for entry in &mut self.strings {
            entry
                .1
                .apply_config(&strings_base_dir.join(format!("{:#x}", entry.0)))?;
        }
        fs::write(base_dir.join("UDC"), &self.udc)?;
        Ok(())
    }

    fn cleanup<P: AsRef<Path>>(base_dir: P) -> error::Result<()>
    where
        Self: Sized,
    {
        let base_dir = base_dir.as_ref();
        if !base_dir.is_dir() {
            return Ok(());
        }
        let udc_path = base_dir.join("UDC");
        if fs::read(&udc_path)? != vec![0xa_u8] {
            fs::write(udc_path, "\n")?;
        }
        for entry in fs::read_dir(base_dir.join("configs"))? {
            let entry = entry.map_err(|err| error::ErrorKind::io(err, "configs"))?;
            let path = entry.path();
            UsbConfiguration::cleanup(&path)?;
        }
        for entry in fs::read_dir(base_dir.join("functions"))? {
            let entry = entry.map_err(|err| error::ErrorKind::io(err, "functions"))?;
            let path = entry.path();
            if let Some(path_file_name) = path.file_name() {
                let path_file_name = Path::new(path_file_name);
                log::debug!("Now clean {}", path.display());
                if path_file_name.starts_with(GadgetInfo::HID) {
                    FunctionHidOpts::cleanup(path)?;
                } else if path_file_name.starts_with(GadgetInfo::MASS_STORAGE) {
                    FunctionMsgOpts::cleanup(path)?;
                } else {
                    FunctionDummyOpts::cleanup(path)?;
                }
            } else {
                Err(util::error::ErrorKind::custom(format!(
                    "Can't get file_name from {path:?}"
                )))?;
            }
        }
        OsDesc::cleanup(&base_dir.join("os_desc"))?;
        for entry in fs::read_dir(base_dir.join("strings"))? {
            let entry = entry.map_err(|err| error::ErrorKind::io(err, "strings"))?;
            let path = entry.path();
            GadgetStrings::cleanup(&path)?;
        }
        fs::remove_dir(base_dir)?;
        Ok(())
    }
}

impl GadgetInfo {
    pub const HID: &'static str = "hid";
    pub const MASS_STORAGE: &'static str = "mass_storage";
}

pub struct UsbConfiguration {
    pub bm_attributes: u8,
    pub max_power: u16,
    pub functions: Vec<String>,
    pub strings: HashMap<u16, GadgetConfigName>,
}

impl Default for UsbConfiguration {
    fn default() -> Self {
        Self {
            bm_attributes: 0x80,
            max_power: 2,
            strings: HashMap::new(),
            functions: Vec::new(),
        }
    }
}

impl Configurable for UsbConfiguration {
    fn apply_config(&mut self, base_dir: &dyn AsRef<Path>) -> error::Result<()> {
        let base_dir = base_dir.as_ref();
        fs::create_dir(base_dir)?;
        fs::write(
            base_dir.join("bmAttributes"),
            self.bm_attributes.to_string(),
        )?;
        fs::write(base_dir.join("MaxPower"), self.max_power.to_string())?;
        let strings_base_dir = base_dir.join("strings");
        for (language_code, gadget_config_name) in &mut self.strings {
            gadget_config_name
                .apply_config(&strings_base_dir.join(format!("{:#x}", language_code)))?;
        }
        for function in &self.functions {
            let function_path = base_dir.join(function);
            symlink(
                base_dir.join(format!("../../functions/{function}")),
                &function_path,
            )
            .map_err(|err| error::ErrorKind::io(err, function_path))?;
        }
        Ok(())
    }
    fn cleanup<P: AsRef<Path>>(base_dir: P) -> error::Result<()>
    where
        Self: Sized,
    {
        let base_dir = base_dir.as_ref();
        if !base_dir.is_dir() {
            return Ok(());
        }
        for entry in fs::read_dir(base_dir)? {
            let entry = entry.map_err(|err| error::ErrorKind::io(err, base_dir))?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.is_symlink() {
                fs::remove_file(&path)?;
            }
        }
        for entry in fs::read_dir(base_dir.join("strings"))? {
            let entry = entry.map_err(|err| error::ErrorKind::io(err, "strings"))?;
            let path = entry.path();
            GadgetConfigName::cleanup(&path)?;
        }
        fs::remove_dir(base_dir)?;
        Ok(())
    }
}

pub struct GadgetStrings {
    pub manufacturer: String,
    pub product: String,
    pub serialnumber: String,
}

impl Default for GadgetStrings {
    fn default() -> Self {
        Self {
            manufacturer: "\n".to_string(),
            product: "\n".to_string(),
            serialnumber: "\n".to_string(),
        }
    }
}

impl Configurable for GadgetStrings {
    fn apply_config(&mut self, base_dir: &dyn AsRef<Path>) -> error::Result<()> {
        let base_dir = base_dir.as_ref();
        fs::create_dir(base_dir)?;
        fs::write(base_dir.join("manufacturer"), &self.manufacturer)?;
        fs::write(base_dir.join("product"), &self.product)?;
        fs::write(base_dir.join("serialnumber"), &self.serialnumber)?;
        Ok(())
    }
}

pub struct OsDesc {
    pub b_vendor_code: u8,
    pub qw_sign: String,
}

impl Default for OsDesc {
    fn default() -> Self {
        Self {
            b_vendor_code: 0,
            qw_sign: "\n".to_string(),
        }
    }
}

impl Configurable for OsDesc {
    fn apply_config(&mut self, base_dir: &dyn AsRef<Path>) -> error::Result<()> {
        let base_dir = base_dir.as_ref();
        fs::write(base_dir.join("use"), "1")?;
        fs::write(
            base_dir.join("b_vendor_code"),
            self.b_vendor_code.to_string(),
        )?;
        fs::write(base_dir.join("qw_sign"), &self.qw_sign)?;
        Ok(())
    }
    fn cleanup<P: AsRef<Path>>(base_dir: P) -> error::Result<()>
    where
        Self: Sized,
    {
        let base_dir = base_dir.as_ref();
        fs::write(base_dir.join("use"), "0")?;
        Ok(())
    }
}

pub struct GadgetConfigName {
    pub configuration: String,
}

impl GadgetConfigName {}

impl Default for GadgetConfigName {
    fn default() -> Self {
        Self {
            configuration: "\n".to_string(),
        }
    }
}

impl Configurable for GadgetConfigName {
    fn apply_config(&mut self, base_dir: &dyn AsRef<Path>) -> error::Result<()> {
        let base_dir = base_dir.as_ref();
        fs::create_dir(base_dir)?;
        fs::write(base_dir.join("configuration"), &self.configuration)?;
        Ok(())
    }
}
