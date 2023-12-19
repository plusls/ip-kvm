use std::{collections::HashMap, path::Path};

use util::{error, fs};

use crate::{Configurable, UsbFunctionOpts};

#[derive(Clone)]
pub struct FunctionMsgOpts {
    pub stall: bool,
    pub luns: HashMap<String, MsgLun>,
}

impl FunctionMsgOpts {
    const LUN_NAME_PREFIX: &'static str = "lun";

    pub fn lun_name(lun_id: u8) -> String {
        format!("{}.{lun_id}", Self::LUN_NAME_PREFIX)
    }
}

impl Default for FunctionMsgOpts {
    fn default() -> Self {
        let mut ret = Self {
            stall: false,
            luns: HashMap::new(),
        };
        ret.luns.insert(Self::lun_name(0), MsgLun::default());
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
        self.from_config(&base_dir)?;
        Ok(())
    }

    fn from_config(&mut self, base_dir: &dyn AsRef<Path>) -> error::Result<()> {
        let base_dir = base_dir.as_ref();
        self.stall = fs::read_to_bool(base_dir.join("stall"))?;

        self.luns.clear();

        for entry in util::fs::read_dir(base_dir)? {
            let entry = entry.map_err(|err| error::ErrorKind::io(err, base_dir))?;
            let path = entry.path();
            if let Some(path_file_name) = path.file_name() {
                if path.is_dir() {
                    let mut msg_lun = MsgLun::default();
                    if let Some(path_file_name) = path_file_name.to_str() {
                        msg_lun.from_config(&path)?;
                        self.luns.insert(path_file_name.into(), msg_lun);
                    }
                }
            }
        }
        Ok(())
    }

    fn cleanup<P: AsRef<Path>>(base_dir: P) -> error::Result<()>
    where
        Self: Sized,
    {
        println!("fuck1");
        let base_dir = base_dir.as_ref();
        if !base_dir.is_dir() {
            return Ok(());
        }
        println!("fuck2");

        log::info!("path: {base_dir:?}");
        for entry in fs::read_dir(base_dir.join("."))? {
            let entry = entry.map_err(|err| error::ErrorKind::io(err, "."))?;
            let path = entry.path();
            log::info!("path: {path:?}");
            if let Some(path_file_name) = path.file_name() {
                if path.is_dir() && path_file_name != Self::lun_name(0).as_str() {
                    MsgLun::cleanup(base_dir.join(path_file_name))?;
                }
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
            ro: false,
        }
    }
}

impl Configurable for MsgLun {
    fn apply_config(&mut self, base_dir: &dyn AsRef<Path>) -> error::Result<()> {
        let base_dir = base_dir.as_ref();

        if !base_dir.is_dir() {
            fs::create_dir(base_dir)?;
        }
        // 先强制弹出 u 盘
        fs::write(base_dir.join("forced_eject"), "1")?;
        fs::write(base_dir.join("cdrom"), if self.cdrom { "1" } else { "0" })?;
        fs::write(base_dir.join("file"), &self.file)?;
        fs::write(base_dir.join("inquiry_string"), &self.inquiry_string)?;
        fs::write(base_dir.join("nofua"), if self.nofua { "1" } else { "0" })?;
        fs::write(
            base_dir.join("removable"),
            if self.removable { "1" } else { "0" },
        )?;
        fs::write(base_dir.join("ro"), if self.ro { "1" } else { "0" })?;
        Ok(())
    }

    fn from_config(&mut self, base_dir: &dyn AsRef<Path>) -> error::Result<()> {
        let base_dir = base_dir.as_ref();
        self.cdrom = fs::read_to_bool(base_dir.join("cdrom"))?;
        self.file = fs::read_to_string(base_dir.join("file"))?;
        self.inquiry_string = fs::read_to_string(base_dir.join("inquiry_string"))?;
        self.nofua = fs::read_to_bool(base_dir.join("nofua"))?;
        self.removable = fs::read_to_bool(base_dir.join("removable"))?;
        Ok(())
    }

    fn cleanup<P: AsRef<Path>>(base_dir: P) -> error::Result<()>
    where
        Self: Sized,
    {
        let base_dir = base_dir.as_ref();
        fs::remove_dir(base_dir)?;
        Ok(())
    }
}
