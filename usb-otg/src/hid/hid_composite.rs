use std::sync::Arc;

use lazy_static::lazy_static;

use nix::fcntl;
use nix::sys::stat::Mode;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::watch::Sender;
use tokio::sync::Mutex;

use crate::async_fd::AsyncFd;
use crate::hid;
use util::error;

pub const HID_COMPOSITE_RECV_LENGTH: usize = 0x21;
pub const HID_COMPOSITE_SEND_LENGTH: usize = 0x23;
pub const HID_REPORT_ID_MOUSE: u8 = 1;
pub const HID_REPORT_ID_KEYBOARD: u8 = 2;

lazy_static! {

        // from https://github.com/NicoHood/HID/blob/master/src/SingleReport/SingleAbsoluteMouse.cpp
        pub static ref HID_COMPOSITE_FHO: hid::FunctionHidOpts = hid::FunctionHidOpts {
            major: 0,
            minor: 0,
            no_out_endpoint: 1,
            subclass: 1, /* Boot Interface SubClass */
            protocol: 1,  /* Keyboard */
            report_length: HID_COMPOSITE_SEND_LENGTH as u16,
            report_desc: vec![
                // Mouse
                0x05, 0x01,     /* USAGE_PAGE (Generic Desktop)           */
                0x09, 0x02,     /* USAGE (Mouse)                       */
                0xa1, 0x01,     /* COLLECTION (Application)               */
                0x85, HID_REPORT_ID_MOUSE,     /* Report ID (HID_REPORT_ID_MOUSE)                 */

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

                // Padding
                0x75, 0x08,     /*   REPORT_SIZE (8)                      */
                0x95, 0x1c,     /*   REPORT_COUNT (0x1c)                     */ // 0x1c+0x6 = 0x22
                0x81, 0x03,     /*   INPUT (Cnst,Var,Abs)                 */

                0xc0,                           /* END_COLLECTION (Physical) */
                0xc0,            /* END_COLLECTION (Application)           */

                // Keyboard
                0x05, 0x01,     /* USAGE_PAGE (Generic Desktop)           */
                0x09, 0x06,     /* USAGE (Keyboard)                       */
                0xa1, 0x01,     /* COLLECTION (Application)               */
                0x85, HID_REPORT_ID_KEYBOARD,     /* Report ID (HID_REPORT_ID_KEYBOARD)                 */
                // Keys
                0x05, 0x07,     /*   USAGE_PAGE (Keyboard)                */
                0x19, 0x00,     /*   USAGE_MINIMUM (Reserved) */
                0x2a, 0xff, 0x00,     /*   USAGE_MAXIMUM (0xff)   */
                0x15, 0x00,     /*   LOGICAL_MINIMUM (0)                  */
                0x25, 0x01,     /*   LOGICAL_MAXIMUM (1)                  */
                0x75, 0x01,     /*   REPORT_SIZE (1)                      */
                0x96, 0x00, 0x01,     /*   REPORT_COUNT (0x100)                     */
                0x81, 0x02,     /*   INPUT (Data,Var,Abs)                 */

                // Sys Control
                0x05, 0x01,        // Usage Page (Generic Desktop Ctrls)
                0x09, 0x80,        // Usage (Sys Control)
                0x19, 0x81,        //   Usage Minimum (Sys Power Down)
                0x29, 0x8f,        //   Usage Maximum (Sys Warm Restart)
                0x15, 0x00,     /*   LOGICAL_MINIMUM (0)                  */
                0x25, 0x01,     /*   LOGICAL_MAXIMUM (1)                  */
                0x75, 0x01,     /*   REPORT_SIZE (1)                      */
                0x95, 0x0f,     /*   REPORT_COUNT (0xf)                     */
                0x81, 0x02,     /*   INPUT (Data,Var,Abs)                 */

                // Padding
                0x75, 0x01,     /*   REPORT_SIZE (1)                      */
                0x95, 0x01,     /*   REPORT_COUNT (1)                     */
                0x81, 0x03,     /*   INPUT (Cnst,Var,Abs)                 */

                // LEDs Output
                0x05, 0x08,     /*   USAGE_PAGE (LEDs)                    */
                0x19, 0x00,     /*   USAGE_MINIMUM (Undefined)            */
                0x2a, 0xff, 0x00,     /*   USAGE_MAXIMUM (0xff)                 */
                0x75, 0x01,     /*   REPORT_SIZE (1)                      */
                0x96, 0x00, 0x01,     /*   REPORT_COUNT (8)                     */
                0x91, 0x02,     /*   OUTPUT (Data,Var,Abs)                */

                0xc0            /* END_COLLECTION                         */
            ],
        };
}

pub struct HidCompositeDevice {
    hid_composite_dev_read: Mutex<AsyncFd>,
    hid_composite_dev_write: Mutex<AsyncFd>,
    pub hid_composite_dev_send_sender: Arc<Sender<[u8; HID_COMPOSITE_SEND_LENGTH]>>,
}

impl HidCompositeDevice {
    // AsyncFd::new must call in tokio async runtime
    pub async fn new(hid_composite_minor: i32) -> error::Result<Self> {
        let hid_composite_dev_name = format!("/dev/hidg{hid_composite_minor}");

        let hid_composite_dev_read = AsyncFd::try_from(
            fcntl::open(
                hid_composite_dev_name.as_str(),
                fcntl::OFlag::O_RDONLY,
                Mode::empty(),
            )
            .map_err(|err| error::ErrorKind::io(err.into(), &hid_composite_dev_name))?,
        )
        .unwrap();

        let hid_composite_dev_write = AsyncFd::try_from(
            fcntl::open(
                hid_composite_dev_name.as_str(),
                fcntl::OFlag::O_WRONLY,
                Mode::empty(),
            )
            .map_err(|err| error::ErrorKind::io(err.into(), &hid_composite_dev_name))?,
        )
        .unwrap();

        let (hid_composite_dev_send_sender, _) =
            tokio::sync::watch::channel([0; HID_COMPOSITE_SEND_LENGTH]);

        let ret = Self {
            hid_composite_dev_read: Mutex::new(hid_composite_dev_read),
            hid_composite_dev_write: Mutex::new(hid_composite_dev_write),
            hid_composite_dev_send_sender: Arc::new(hid_composite_dev_send_sender),
        };

        Ok(ret)
    }

    pub async fn recv(&self) -> error::Result<[u8; HID_COMPOSITE_RECV_LENGTH]> {
        let mut hid_composite_recv_data = [0_u8; HID_COMPOSITE_RECV_LENGTH];
        let mut hid_composite_dev_read: tokio::sync::MutexGuard<'_, AsyncFd> =
            self.hid_composite_dev_read.lock().await;
        let read_len = hid_composite_dev_read
            .read(&mut hid_composite_recv_data)
            .await
            .map_err(|err| error::ErrorKind::io(err, "hid_composite_dev"))?;
        if read_len != HID_COMPOSITE_RECV_LENGTH {
            log::warn!(
                "hid_composite_dev ignore: {:?}",
                &hid_composite_recv_data[..read_len]
            );
            Err(error::ErrorKind::Ignore)?;
        }
        return Ok(hid_composite_recv_data);
    }

    pub async fn send(&self, hid_composite_send_data: &[u8]) -> error::Result<()> {
        log::debug!("hid_composite_dev send {hid_composite_send_data:?}");
        let mut hid_composite_dev = self.hid_composite_dev_write.lock().await;
        hid_composite_dev
            .write_all(&hid_composite_send_data)
            .await
            .map_err(|err| error::ErrorKind::io(err, "hid_composite_dev"))?;

        Ok(())
    }
}
