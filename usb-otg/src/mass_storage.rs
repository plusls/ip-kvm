use std::{
    collections::HashMap,
    path::Path,
};

use util::{error, fs};

use crate::{Configurable, UsbFunctionOpts};

#[derive(Clone)]
pub struct FunctionMsgOpts {
    pub stall: bool,
    pub luns: HashMap<String, MsgLun>,
}

impl FunctionMsgOpts {
    pub const LUN_0: &'static str = "lun.0";
}

impl Default for FunctionMsgOpts {
    fn default() -> Self {
        let mut ret = Self {
            stall: false,
            luns: HashMap::new(),
        };
        ret.luns.insert(Self::LUN_0.into(), MsgLun::default());
        ret
    }
}

impl UsbFunctionOpts for FunctionMsgOpts {}


impl Configurable for FunctionMsgOpts {
    fn apply_config(&mut self, base_dir: &dyn AsRef<Path>) -> error::Result<()> {
        let base_dir = base_dir.as_ref();
        fs::create_dir(base_dir)?;
        fs::write(base_dir.join("stall"), if self.stall { "1" } else { "0" })?;
        for (lun_name, lun) in &mut self.luns {
            lun.apply_config(&base_dir.join(lun_name))?;
        }
        Ok(())
    }

    fn cleanup<P: AsRef<Path>>(base_dir: P) -> error::Result<()> where Self: Sized {
        let base_dir = base_dir.as_ref();
        if !base_dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(base_dir.join("."))? {
            let entry = entry.map_err(|err| error::ErrorKind::fs(err, "."))?;
            let path = entry.path();

            if path.starts_with("lun.") {
                MsgLun::cleanup(&path)?;
            }
        }

        fs::remove_dir(base_dir)?;
        Ok(())
    }
}


#[derive(Clone)]
pub struct MsgLun {
    pub cdrom: bool,
    pub file: String,
    pub inquiry_string: String,
    pub nofua: bool,
    pub removable: bool,
    pub ro: bool,
}

impl Default for MsgLun {
    fn default() -> Self {
        Self {
            cdrom: false,
            file: "\n".into(),
            inquiry_string: "\n".into(),
            nofua: false,
            removable: true,
            ro: true,
        }
    }
}

impl Configurable for MsgLun {
    fn apply_config(&mut self, base_dir: &dyn AsRef<Path>) -> error::Result<()> {
        let base_dir = base_dir.as_ref();
        if !base_dir.ends_with(FunctionMsgOpts::LUN_0) {
            fs::create_dir(base_dir)?;
        }
        fs::write(base_dir.join("cdrom"), if self.cdrom { "1" } else { "0" })?;
        fs::write(base_dir.join("file"), &self.file)?;
        fs::write(base_dir.join("inquiry_string"), &self.inquiry_string)?;
        fs::write(base_dir.join("nofua"), if self.nofua { "1" } else { "0" })?;
        fs::write(base_dir.join("removable"), if self.removable { "1" } else { "0" })?;
        fs::write(base_dir.join("ro"), if self.ro { "1" } else { "0" })?;
        Ok(())
    }

    fn cleanup<P: AsRef<Path>>(base_dir: P) -> error::Result<()> where Self: Sized {
        let base_dir = base_dir.as_ref();
        if !base_dir.is_dir() {
            return Ok(());
        }
        if !base_dir.ends_with(FunctionMsgOpts::LUN_0) {
            fs::remove_dir(base_dir)?;
        }
        Ok(())
    }
}
