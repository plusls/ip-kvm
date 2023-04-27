use std::os::fd::AsRawFd;

use lazy_static::lazy_static;
use nix::fcntl;
use nix::sys::stat::Mode;
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;
use util::error;

use crate::async_fd::AsyncFd;
use crate::hid;

lazy_static! {

    // from https://github.com/NicoHood/HID/blob/master/src/SingleReport/BootMouse.cpp
    pub static ref MOUSE_LEGACY_FHO: hid::FunctionHidOpts = hid::FunctionHidOpts {
        major: 0,
        minor: 0,
        no_out_endpoint: 1,
        subclass: 1, /* Boot Interface SubClass */
        protocol: 2,  /* Mouse */
        report_length: 4,
        report_desc: vec![
            // Mouse
            0x05, 0x01,     /* USAGE_PAGE (Generic Desktop)           */
            0x09, 0x02,     /* USAGE (Mouse)                       */
            0xa1, 0x01,     /* COLLECTION (Application)               */

            /* Pointer and Physical are required by Apple Recovery */
            0x09, 0x01,                      /*   USAGE (Pointer) */
            0xa1, 0x00,                      /*   COLLECTION (Physical) */

            // 8 Buttons
            0x05, 0x09,     /*   USAGE_PAGE (Button)                */
            0x19, 0x01,     /*   USAGE_MINIMUM (Button 1) */
            0x29, 0x08,     /*   USAGE_MAXIMUM (Button 8)   */
            0x15, 0x00,     /*   LOGICAL_MINIMUM (0)                  */
            0x25, 0x01,     /*   LOGICAL_MAXIMUM (1)                  */
            0x75, 0x01,     /*   REPORT_SIZE (1)                      */
            0x95, 0x08,     /*   REPORT_COUNT (8)                     */
            0x81, 0x02,     /*   INPUT (Data,Var,Abs)                 */

            /* X, Y, Wheel */
            0x05, 0x01,                      /*     USAGE_PAGE (Generic Desktop) */
            0x09, 0x30,                      /*     USAGE (X) */
            0x09, 0x31,                      /*     USAGE (Y) */
            0x09, 0x38,                      /*     USAGE (Wheel) */
            0x15, 0x81,                      /*     LOGICAL_MINIMUM (-127) */
            0x25, 0x7f,                      /*     LOGICAL_MAXIMUM (127) */
            0x75, 0x08,                      /*     REPORT_SIZE (8) */
            0x95, 0x03,                      /*     REPORT_COUNT (3) */
            0x81, 0x06,                      /*     INPUT (Data,Var,Rel) */

            /* End */
            0xc0,                           /* END_COLLECTION (Physical) */
            0xc0            /* END_COLLECTION                         */
        ],
    };

    // from https://github.com/NicoHood/HID/blob/master/src/SingleReport/SingleAbsoluteMouse.cpp
    pub static ref MOUSE_FHO: hid::FunctionHidOpts = hid::FunctionHidOpts {
        major: 0,
        minor: 0,
        no_out_endpoint: 1,
        subclass: 1, /* Boot Interface SubClass */
        protocol: 2,  /* Mouse */
        report_length: 6,
        report_desc: vec![
            // Mouse
            0x05, 0x01,     /* USAGE_PAGE (Generic Desktop)           */
            0x09, 0x02,     /* USAGE (Mouse)                       */
            0xa1, 0x01,     /* COLLECTION (Application)               */

            /* Pointer and Physical are required by Apple Recovery */
            0x09, 0x01,                      /*   USAGE (Pointer) */
            0xa1, 0x00,                      /*   COLLECTION (Physical) */

            // 8 Buttons
            0x05, 0x09,     /*   USAGE_PAGE (Button)                */
            0x19, 0x01,     /*   USAGE_MINIMUM (Button 1) */
            0x29, 0x08,     /*   USAGE_MAXIMUM (Button 8)   */
            0x15, 0x00,     /*   LOGICAL_MINIMUM (0)                  */
            0x25, 0x01,     /*   LOGICAL_MAXIMUM (1)                  */
            0x75, 0x01,     /*   REPORT_SIZE (1)                      */
            0x95, 0x08,     /*   REPORT_COUNT (8)                     */
            0x81, 0x02,     /*   INPUT (Data,Var,Abs)                 */

            // X, Y
            0x05, 0x01,     /* USAGE_PAGE (Generic Desktop)           */
            0x09, 0x30,     /* USAGE (X)                       */
            0x09, 0x31,     /* USAGE (Y)                       */
            0x16, 0x00, 0x00,				 /* 	Logical Minimum (0); NOTE: Windows 7 can't handle negative value */
            0x26, 0xFF, 0x7F,				 /* 	Logical Maximum (32767) */
            0x75, 0x10,						 /* 	Report Size (16), */
            0x95, 0x02,						 /* 	Report Count (2), */
            0x81, 0x02,						 /* 	Input (Data, Variable, Absolute) */

            /* Wheel */
            0x09, 0x38,                      /*     USAGE (Wheel) */
            0x15, 0x81,                      /*     LOGICAL_MINIMUM (-127) */
            0x25, 0x7f,                      /*     LOGICAL_MAXIMUM (127) */
            0x75, 0x08,                      /*     REPORT_SIZE (8) */
            0x95, 0x01,                      /*     REPORT_COUNT (1) */
            0x81, 0x06,                      /*     INPUT (Data,Var,Rel) */

            0xc0,                           /* END_COLLECTION (Physical) */
            0xc0            /* END_COLLECTION                         */
        ],
    };
}

#[derive(Default)]
pub struct Mouse {
    pub button: u8,
}


impl Mouse {
    pub const ABS_MAX: u16 = 0x7fff;

    pub fn clear(&mut self) {
        self.button = 0;
    }

    pub fn get_button(&self, button_id: u16) -> bool {
        if button_id > 8 || button_id == 0 {
            return false;
        }
        return (self.button >> (button_id - 1) as u8) & 1 == 1;
    }

    pub fn set_button(&mut self, button_id: u16, status: bool) -> bool {
        if button_id > 8 || button_id == 0 {
            return false;
        }
        let prev = self.button;
        if status {
            self.button |= 1 << (button_id - 1) as u8;
        } else {
            self.button &= !(1 << (button_id - 1) as u8);
        }
        prev != self.button
    }

    pub fn get_payload(&self, mut x: u16, mut y: u16, mut wheel: i8) -> [u8; 6] {
        if x > Self::ABS_MAX {
            x = Self::ABS_MAX;
        }
        if y > Self::ABS_MAX {
            y = Self::ABS_MAX;
        }
        if wheel == -128 {
            wheel = -127;
        }

        let mut ret = [0; 6];
        ret[0] = self.button;
        ret[1..3].copy_from_slice(&x.to_le_bytes());
        ret[3..5].copy_from_slice(&y.to_le_bytes());
        ret[5..6].copy_from_slice(&wheel.to_le_bytes());
        ret
    }

    pub fn get_legacy_payload(&self, mut x: i8, mut y: i8, mut wheel: i8) -> [u8; 4] {
        if x == -128 {
            x = -127;
        }
        if y == -128 {
            y = -127;
        }
        if wheel == -128 {
            wheel = -127;
        }

        let mut ret = [0; 4];
        ret[0] = self.button;
        ret[1..2].copy_from_slice(&x.to_le_bytes());
        ret[2..3].copy_from_slice(&y.to_le_bytes());
        ret[3..4].copy_from_slice(&wheel.to_le_bytes());
        ret
    }
}


pub struct MouseDevice {
    pub mouse: Mutex<Mouse>,
    mouse_dev_write: Mutex<AsyncFd>,
    mouse_legacy_dev_write: Mutex<AsyncFd>,
}

impl MouseDevice {
    pub async fn new(mouse_minor: i32, mouse_legacy_minor: i32) -> error::Result<Self> {
        let mouse_dev_name = format!("/dev/hidg{mouse_minor}");
        let mouse_legacy_dev_name = format!("/dev/hidg{mouse_legacy_minor}");

        let mouse_dev_write =
            AsyncFd::try_from(fcntl::open(mouse_dev_name.as_str(), fcntl::OFlag::O_WRONLY, Mode::empty())
                .map_err(|err| error::ErrorKind::fs(err.into(), &mouse_dev_name))?
            ).unwrap();

        let mouse_legacy_dev_write =
            AsyncFd::try_from(fcntl::open(mouse_legacy_dev_name.as_str(), fcntl::OFlag::O_WRONLY, Mode::empty())
                .map_err(|err| error::ErrorKind::fs(err.into(), &mouse_legacy_dev_name))?
            ).unwrap();

        let ret = Self {
            mouse: Default::default(),
            mouse_dev_write: Mutex::new(mouse_dev_write),
            mouse_legacy_dev_write: Mutex::new(mouse_legacy_dev_write),
        };

        Ok(ret)
    }

    pub async fn set_button(&self, button_id: u16, status: bool) -> bool {
        return self.mouse.lock().await
            .set_button(button_id, status);
    }

    pub async fn send(&self, x: u16, y: u16, wheel: i8) -> error::Result<()> {
        let mut mouse_dev_write = self.mouse_dev_write.lock().await;
        let payload = self.mouse.lock().await.get_payload(x, y, wheel);
        println!("mouse send {payload:?} {}", mouse_dev_write.as_raw_fd());
        mouse_dev_write.write_all(&payload).await
            .map_err(|err| error::ErrorKind::fs(err, "mouse_dev write_all"))?;
        Ok(())
    }
    pub async fn send_legacy(&self, x: i8, y: i8, wheel: i8) -> error::Result<()> {
        let mut mouse_legacy_dev = self.mouse_legacy_dev_write.lock().await;
        let payload = self.mouse.lock().await.get_legacy_payload(x, y, wheel);
        println!("mouse send_legacy {payload:?}");
        mouse_legacy_dev.write_all(&payload).await
            .map_err(|err| error::ErrorKind::fs(err, "mouse_legacy_dev write_all"))?;
        Ok(())
    }
}