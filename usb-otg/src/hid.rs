use std::path::Path;
use std::str::FromStr;

use lazy_static::lazy_static;
use regex::Regex;
use util::fs;

use crate::{Configurable, error, UsbFunctionOpts};

pub mod keyboard;
pub mod generic_desktop;

#[derive(Clone)]
pub struct FunctionHidOpts {
    // read only
    pub major: i32,
    pub minor: i32,
    // write only
    pub no_out_endpoint: u8,
    pub protocol: u8,
    pub report_desc: Vec<u8>,
    pub report_length: u16,
    pub subclass: u8,
}

impl Default for FunctionHidOpts {
    fn default() -> Self {
        Self {
            major: 0,
            minor: 0,
            no_out_endpoint: 0,
            protocol: 0,
            report_desc: Vec::new(),
            report_length: 0,
            subclass: 0,
        }
    }
}

impl Configurable for FunctionHidOpts {
    fn apply_config(&mut self, base_dir: &dyn AsRef<Path>) -> error::Result<()> {
        let base_dir = base_dir.as_ref();
        fs::create_dir(base_dir)?;
        self.read_dev(&base_dir)?;
        // 低版本内核可能没这个
        let _ = fs::write(base_dir.join("no_out_endpoint"), self.no_out_endpoint.to_string());
        fs::write(base_dir.join("protocol"), self.protocol.to_string())?;
        fs::write(base_dir.join("report_desc"), &self.report_desc)?;
        fs::write(base_dir.join("report_length"), self.report_length.to_string())?;
        fs::write(base_dir.join("subclass"), self.subclass.to_string())?;
        Ok(())
    }
    fn from_config(base_dir: &dyn AsRef<Path>) -> error::Result<Self> where Self: Sized {
        let base_dir = base_dir.as_ref();
        let mut ret = Self {
            major: 0,
            minor: 0,
            // 内核保证了 no_out_endpoint 的数据一定是合法的
            no_out_endpoint: u8::from_str(&fs::read_to_string(base_dir.join("no_out_endpoint")).unwrap_or("0".into())).unwrap(),
            protocol: u8::from_str(&fs::read_to_string(base_dir.join("protocol"))?).unwrap(),
            report_desc: fs::read(base_dir.join("report_desc"))?,
            report_length: u16::from_str(&fs::read_to_string(base_dir.join("report_length"))?).unwrap(),
            subclass: u8::from_str(&fs::read_to_string(base_dir.join("subclass"))?).unwrap(),
        };
        ret.read_dev(&base_dir)?;
        Ok(ret)
    }
}

impl UsbFunctionOpts for FunctionHidOpts {}

impl FunctionHidOpts {
    fn read_dev(&mut self, base_dir: &dyn AsRef<Path>) -> error::Result<()> {
        let base_dir = base_dir.as_ref();
        lazy_static! {
            static ref RE_DEV_MATCH: Regex =
                Regex::new(r"^(\d+):(\d+)$").unwrap();
        }
        let dev_string = fs::read_to_string(base_dir.join("dev"))?;
        let res = RE_DEV_MATCH.captures(dev_string.trim());
        if let Some(res) = res {
            self.major = i32::from_str(&res[1]).unwrap();
            self.minor = i32::from_str(&res[2]).unwrap();
            Ok(())
        } else {
            unreachable!()
        }
    }
}