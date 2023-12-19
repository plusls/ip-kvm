use std::sync::Arc;

use lazy_static::lazy_static;
use nix::fcntl;
use nix::sys::stat::Mode;
use tokio::io::AsyncWriteExt;
use tokio::sync::watch::Sender;
use tokio::sync::Mutex;
use util::error;

use crate::async_fd::AsyncFd;
use crate::hid::hid_composite;
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


}

#[derive(Default)]
pub struct Mouse {
    pub button: u8,
}

impl Mouse {
    pub const ABS_MAX: u16 = 0x7fff;
    pub const REL_MIN: i8 = -127;
    pub const WHEEL_MIN: i8 = -127;

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
        if wheel < Self::WHEEL_MIN {
            wheel = Self::WHEEL_MIN;
        }

        let mut ret = [0; 6];
        ret[0] = self.button;
        ret[1..3].copy_from_slice(&x.to_le_bytes());
        ret[3..5].copy_from_slice(&y.to_le_bytes());
        ret[5] = wheel as u8;
        ret
    }

    pub fn get_legacy_payload(&self, mut x: i8, mut y: i8, mut wheel: i8) -> [u8; 4] {
        if x < Self::REL_MIN {
            x = Self::REL_MIN;
        }
        if y < Self::REL_MIN {
            y = Self::REL_MIN;
        }
        if wheel < Self::WHEEL_MIN {
            wheel = Self::WHEEL_MIN;
        }

        let mut ret = [0; 4];
        ret[0] = self.button;
        ret[1] = x as u8;
        ret[2] = y as u8;
        ret[3] = wheel as u8;
        ret
    }
}

pub struct MouseDevice {
    pub mouse: Mutex<Mouse>,
    mouse_legacy_dev_write: Mutex<AsyncFd>,
    hid_composite_dev_send_sender: Arc<Sender<[u8; hid_composite::HID_COMPOSITE_SEND_LENGTH]>>,
}

impl MouseDevice {
    pub async fn new(
        mouse_legacy_minor: i32,
        hid_composite_dev_send_sender: Arc<Sender<[u8; hid_composite::HID_COMPOSITE_SEND_LENGTH]>>,
    ) -> error::Result<Self> {
        let mouse_legacy_dev_name = format!("/dev/hidg{mouse_legacy_minor}");

        let mouse_legacy_dev_write = AsyncFd::try_from(
            fcntl::open(
                mouse_legacy_dev_name.as_str(),
                fcntl::OFlag::O_WRONLY,
                Mode::empty(),
            )
            .map_err(|err| error::ErrorKind::io(err.into(), &mouse_legacy_dev_name))?,
        )
        .unwrap();

        let ret = Self {
            mouse: Default::default(),
            mouse_legacy_dev_write: Mutex::new(mouse_legacy_dev_write),
            hid_composite_dev_send_sender,
        };

        Ok(ret)
    }

    pub async fn set_button(&self, button_id: u16, status: bool) -> bool {
        return self.mouse.lock().await.set_button(button_id, status);
    }

    pub async fn send(&self, x: u16, y: u16, wheel: i8) -> error::Result<()> {
        let mut payload = [0_u8; hid_composite::HID_COMPOSITE_SEND_LENGTH];
        payload[0] = hid_composite::HID_REPORT_ID_MOUSE;
        payload[1..1+6].copy_from_slice(&self.mouse.lock().await.get_payload(x, y, wheel));
        log::debug!("hid_composite_dev send mouse {payload:?}");
        self.hid_composite_dev_send_sender.send(payload).unwrap();
        Ok(())
    }

    pub async fn send_legacy(&self, x: i8, y: i8, wheel: i8) -> error::Result<()> {
        let mut mouse_legacy_dev = self.mouse_legacy_dev_write.lock().await;
        let payload = self.mouse.lock().await.get_legacy_payload(x, y, wheel);
        log::debug!("mouse send_legacy {payload:?}");
        mouse_legacy_dev
            .write_all(&payload)
            .await
            .map_err(|err| error::ErrorKind::io(err, "mouse_legacy_dev write_all"))?;
        Ok(())
    }
}
