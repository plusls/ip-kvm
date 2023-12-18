use lazy_static::lazy_static;
use nix::fcntl;
use nix::sys::stat::Mode;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::sync::watch::Sender;
use util::error;

use crate::async_fd::AsyncFd;
use crate::hid;
use crate::hid::generic_desktop;

pub mod usage_id {
    pub const KEYBOARD_ERROR_ROLL_OVER: u16 = 0x1;
    pub const KEYBOARD_POST_FAIL: u16 = 0x2;
    pub const KEYBOARD_ERROR_UNDEFINED: u16 = 0x3;
    pub const KEYBOARD_A: u16 = 0x4;
    pub const KEYBOARD_B: u16 = 0x5;
    pub const KEYBOARD_C: u16 = 0x6;
    pub const KEYBOARD_D: u16 = 0x7;
    pub const KEYBOARD_E: u16 = 0x8;
    pub const KEYBOARD_F: u16 = 0x9;
    pub const KEYBOARD_G: u16 = 0xa;
    pub const KEYBOARD_H: u16 = 0xb;
    pub const KEYBOARD_I: u16 = 0xc;
    pub const KEYBOARD_J: u16 = 0xd;
    pub const KEYBOARD_K: u16 = 0xe;
    pub const KEYBOARD_L: u16 = 0xf;
    pub const KEYBOARD_M: u16 = 0x10;
    pub const KEYBOARD_N: u16 = 0x11;
    pub const KEYBOARD_O: u16 = 0x12;
    pub const KEYBOARD_P: u16 = 0x13;
    pub const KEYBOARD_Q: u16 = 0x14;
    pub const KEYBOARD_R: u16 = 0x15;
    pub const KEYBOARD_S: u16 = 0x16;
    pub const KEYBOARD_T: u16 = 0x17;
    pub const KEYBOARD_U: u16 = 0x18;
    pub const KEYBOARD_V: u16 = 0x19;
    pub const KEYBOARD_W: u16 = 0x1a;
    pub const KEYBOARD_X: u16 = 0x1b;
    pub const KEYBOARD_Y: u16 = 0x1c;
    pub const KEYBOARD_Z: u16 = 0x1d;
    pub const KEYBOARD_1: u16 = 0x1e;
    pub const KEYBOARD_2: u16 = 0x1f;
    pub const KEYBOARD_3: u16 = 0x20;
    pub const KEYBOARD_4: u16 = 0x21;
    pub const KEYBOARD_5: u16 = 0x22;
    pub const KEYBOARD_6: u16 = 0x23;
    pub const KEYBOARD_7: u16 = 0x24;
    pub const KEYBOARD_8: u16 = 0x25;
    pub const KEYBOARD_9: u16 = 0x26;
    pub const KEYBOARD_0: u16 = 0x27;
    pub const KEYBOARD_ENTER: u16 = 0x28;
    pub const KEYBOARD_ESCAPE: u16 = 0x29;
    pub const KEYBOARD_BACKSPACE: u16 = 0x2a;
    pub const KEYBOARD_TAB: u16 = 0x2b;
    pub const KEYBOARD_SPACEBAR: u16 = 0x2c;
    pub const KEYBOARD_MINUS: u16 = 0x2d;
    pub const KEYBOARD_EQUAL: u16 = 0x2e;
    pub const KEYBOARD_LEFT_BRACKET: u16 = 0x2f;
    pub const KEYBOARD_RIGHT_BRACKED: u16 = 0x30;
    pub const KEYBOARD_REVERSE_SOLIDUS: u16 = 0x31;
    // pub const KEYBOARD_SHARP :u16 = 0x32;
    pub const KEYBOARD_SEMICOLON: u16 = 0x33;
    pub const KEYBOARD_SINGLE_QUOTE: u16 = 0x34;
    pub const KEYBOARD_GRAVE_ACCENT: u16 = 0x35;
    pub const KEYBOARD_COMMA: u16 = 0x36;
    pub const KEYBOARD_DOT: u16 = 0x37;
    pub const KEYBOARD_SOLIDUS: u16 = 0x38;
    pub const KEYBOARD_CAPS_LOCK: u16 = 0x39;
    pub const KEYBOARD_F1: u16 = 0x3a;
    pub const KEYBOARD_F2: u16 = 0x3b;
    pub const KEYBOARD_F3: u16 = 0x3c;
    pub const KEYBOARD_F4: u16 = 0x3d;
    pub const KEYBOARD_F5: u16 = 0x3e;
    pub const KEYBOARD_F6: u16 = 0x3f;
    pub const KEYBOARD_F7: u16 = 0x40;
    pub const KEYBOARD_F8: u16 = 0x41;
    pub const KEYBOARD_F9: u16 = 0x42;
    pub const KEYBOARD_F10: u16 = 0x43;
    pub const KEYBOARD_F11: u16 = 0x44;
    pub const KEYBOARD_F12: u16 = 0x45;
    pub const KEYBOARD_PRINT_SCREEN: u16 = 0x46;
    pub const KEYBOARD_SCROLL_LOCK: u16 = 0x47;
    pub const KEYBOARD_PAUSE: u16 = 0x48;
    pub const KEYBOARD_INSERT: u16 = 0x49;
    pub const KEYBOARD_HOME: u16 = 0x4a;
    pub const KEYBOARD_PAGEUP: u16 = 0x4b;
    pub const KEYBOARD_DELETE: u16 = 0x4c;
    pub const KEYBOARD_END: u16 = 0x4d;
    pub const KEYBOARD_PAGE_DOWN: u16 = 0x4e;
    pub const KEYBOARD_RIGHT_ARROW: u16 = 0x4f;
    pub const KEYBOARD_LEFT_ARROW: u16 = 0x50;
    pub const KEYBOARD_DOWN_ARROW: u16 = 0x51;
    pub const KEYBOARD_UP_ARROW: u16 = 0x52;
    pub const KEYPAD_NUM_LOCK: u16 = 0x53;
    pub const KEYPAD_SOLIDUS: u16 = 0x54;
    pub const KEYPAD_STAR: u16 = 0x55;
    pub const KEYPAD_MINUS: u16 = 0x56;
    pub const KEYPAD_PLUS: u16 = 0x57;
    pub const KEYPAD_ENTER: u16 = 0x58;
    pub const KEYPAD_1: u16 = 0x59;
    pub const KEYPAD_2: u16 = 0x5a;
    pub const KEYPAD_3: u16 = 0x5b;
    pub const KEYPAD_4: u16 = 0x5c;
    pub const KEYPAD_5: u16 = 0x5d;
    pub const KEYPAD_6: u16 = 0x5e;
    pub const KEYPAD_7: u16 = 0x5f;
    pub const KEYPAD_8: u16 = 0x60;
    pub const KEYPAD_9: u16 = 0x61;
    pub const KEYPAD_0: u16 = 0x62;
    pub const KEYPAD_DOT: u16 = 0x63;
    // pub const KEYBOARD_REVERSE_SOLIDUS :u16 = 0x64;
    pub const KEYBOARD_APPLICATION: u16 = 0x65;
    pub const KEYBOARD_POWER: u16 = 0x66;
    pub const KEYPAD_EQUAL: u16 = 0x67;
    pub const KEYBOARD_F13: u16 = 0x68;
    pub const KEYBOARD_F14: u16 = 0x69;
    pub const KEYBOARD_F15: u16 = 0x6a;
    pub const KEYBOARD_F16: u16 = 0x6b;
    pub const KEYBOARD_F17: u16 = 0x6c;
    pub const KEYBOARD_F18: u16 = 0x6d;
    pub const KEYBOARD_F19: u16 = 0x6e;
    pub const KEYBOARD_F20: u16 = 0x6f;
    pub const KEYBOARD_F21: u16 = 0x70;
    pub const KEYBOARD_F22: u16 = 0x71;
    pub const KEYBOARD_F23: u16 = 0x72;
    pub const KEYBOARD_F24: u16 = 0x73;
    pub const KEYBOARD_EXECUTE: u16 = 0x74;
    pub const KEYBOARD_HELP: u16 = 0x75;
    pub const KEYBOARD_MENU: u16 = 0x76;
    pub const KEYBOARD_SELECT: u16 = 0x77;
    pub const KEYBOARD_STOP: u16 = 0x78;
    pub const KEYBOARD_AGAIN: u16 = 0x79;
    pub const KEYBOARD_UNDO: u16 = 0x7a;
    pub const KEYBOARD_CUT: u16 = 0x7b;
    pub const KEYBOARD_COPY: u16 = 0x7c;
    pub const KEYBOARD_PASTE: u16 = 0x7d;
    pub const KEYBOARD_FIND: u16 = 0x7e;
    pub const KEYBOARD_MUTE: u16 = 0x7f;
    pub const KEYBOARD_VOLUME_UP: u16 = 0x80;
    pub const KEYBOARD_VOLUME_DOWN: u16 = 0x81;
    pub const KEYBOARD_LOCKING_CAPS_LOCK: u16 = 0x82;
    pub const KEYBOARD_LOCKING_NUM_LOCK: u16 = 0x83;
    pub const KEYBOARD_LOCKING_SCROLL_LOCK: u16 = 0x84;
    pub const KEYPAD_COMMA: u16 = 0x85;
    pub const KEYPAD_EQUAL_SIGN: u16 = 0x86;
    pub const KEYBOARD_INTERNATIONAL1: u16 = 0x87;
    pub const KEYBOARD_INTERNATIONAL2: u16 = 0x88;
    pub const KEYBOARD_INTERNATIONAL3: u16 = 0x89;
    pub const KEYBOARD_INTERNATIONAL4: u16 = 0x8a;
    pub const KEYBOARD_INTERNATIONAL5: u16 = 0x8b;
    pub const KEYBOARD_INTERNATIONAL6: u16 = 0x8c;
    pub const KEYBOARD_INTERNATIONAL7: u16 = 0x8d;
    pub const KEYBOARD_INTERNATIONAL8: u16 = 0x8e;
    pub const KEYBOARD_INTERNATIONAL9: u16 = 0x8f;
    pub const KEYBOARD_LANG1: u16 = 0x90;
    pub const KEYBOARD_LANG2: u16 = 0x91;
    pub const KEYBOARD_LANG3: u16 = 0x92;
    pub const KEYBOARD_LANG4: u16 = 0x93;
    pub const KEYBOARD_LANG5: u16 = 0x94;
    pub const KEYBOARD_LANG6: u16 = 0x95;
    pub const KEYBOARD_LANG7: u16 = 0x96;
    pub const KEYBOARD_LANG8: u16 = 0x97;
    pub const KEYBOARD_LANG9: u16 = 0x98;
    pub const KEYBOARD_ALTERNATE_ERASE: u16 = 0x99;
    pub const KEYBOARD_SYSREQ: u16 = 0x9a;
    pub const KEYBOARD_CANCEL: u16 = 0x9b;
    pub const KEYBOARD_CLEAR: u16 = 0x9c;
    pub const KEYBOARD_PRIOR: u16 = 0x9d;
    pub const KEYBOARD_RETURN: u16 = 0x9e;
    pub const KEYBOARD_SEPARATOR: u16 = 0x9f;
    pub const KEYBOARD_OUT: u16 = 0xa0;
    pub const KEYBOARD_OPER: u16 = 0xa1;
    // pub const KEYBOARD_CLEAR :u16 = 0xa2;
    pub const KEYBOARD_CRSEL: u16 = 0xa3;
    pub const KEYBOARD_EXSEL: u16 = 0xa4;
    pub const KEYPAD_00: u16 = 0xb0;
    pub const KEYPAD_000: u16 = 0xb1;
    pub const THOUSANDS_SEPARATOR: u16 = 0xb2;
    pub const DECIMAL_SEPARATOR: u16 = 0xb3;
    pub const CURRENCY_UNIT: u16 = 0xb4;
    pub const CURRENCY_SUB_UNIT: u16 = 0xb5;
    pub const KEYPAD_LEFT_PARENTHESIS: u16 = 0xb6;
    pub const KEYPAD_RIGHT_PARENTHESIS: u16 = 0xb7;
    pub const KEYPAD_LEFT_BRACE: u16 = 0xb8;
    pub const KEYPAD_RIGHT_BRACE: u16 = 0xb9;
    pub const KEYPAD_TAB: u16 = 0xba;
    pub const KEYPAD_BACKSPACE: u16 = 0xbb;
    pub const KEYPAD_A: u16 = 0xbc;
    pub const KEYPAD_B: u16 = 0xbd;
    pub const KEYPAD_C: u16 = 0xbe;
    pub const KEYPAD_D: u16 = 0xbf;
    pub const KEYPAD_E: u16 = 0xc0;
    pub const KEYPAD_F: u16 = 0xc1;
    pub const KEYPAD_XOR: u16 = 0xc2;
    pub const KEYPAD_CARET: u16 = 0xc3;
    pub const KEYPAD_PERCENT: u16 = 0xc4;
    pub const KEYPAD_LEFT_ANGLE_BRACKETS: u16 = 0xc5;
    pub const KEYPAD_RIGHT_ANGLE_BRACKETS: u16 = 0xc6;
    pub const KEYPAD_AND: u16 = 0xc7;
    pub const KEYPAD_LOGICAL_AND: u16 = 0xc8;
    pub const KEYPAD_OR: u16 = 0xc9;
    pub const KEYPAD_LOGICAL_OR: u16 = 0xca;
    pub const KEYPAD_COLON: u16 = 0xcb;
    pub const KEYPAD_SHARP: u16 = 0xcc;
    pub const KEYPAD_SPACE: u16 = 0xcd;
    pub const KEYPAD_AT: u16 = 0xce;
    pub const KEYPAD_EXCLAMATION: u16 = 0xcf;
    pub const KEYPAD_MEMORY_STORE: u16 = 0xd0;
    pub const KEYPAD_MEMORY_RECALL: u16 = 0xd1;
    pub const KEYPAD_MEMORY_CLEAR: u16 = 0xd2;
    pub const KEYPAD_MEMORY_ADD: u16 = 0xd3;
    pub const KEYPAD_MEMORY_SUBTRACT: u16 = 0xd4;
    pub const KEYPAD_MEMORY_MULTIPLY: u16 = 0xd5;
    pub const KEYPAD_MEMORY_DIVIDE: u16 = 0xd6;
    // pub const KEYPAD_PLUS :u16 = 0xd7;
    pub const KEYPAD_CLEAR: u16 = 0xd8;
    pub const KEYPAD_CLEAR_ENTRY: u16 = 0xd9;
    pub const KEYPAD_BINARY: u16 = 0xda;
    pub const KEYPAD_OCTAL: u16 = 0xdb;
    pub const KEYPAD_DECIMAL: u16 = 0xdc;
    pub const KEYPAD_HEXADECIMAL: u16 = 0xdd;
    pub const KEYBOARD_LEFT_CONTROL: u16 = 0xe0;
    pub const KEYBOARD_LEFT_SHIFT: u16 = 0xe1;
    pub const KEYBOARD_LEFT_ALT: u16 = 0xe2;
    pub const KEYBOARD_LEFT_GUI: u16 = 0xe3;
    pub const KEYBOARD_RIGHT_CONTROL: u16 = 0xe4;
    pub const KEYBOARD_RIGHT_SHIFT: u16 = 0xe5;
    pub const KEYBOARD_RIGHT_ALT: u16 = 0xe6;
    pub const KEYBOARD_RIGHT_GUI: u16 = 0xe7;
}

lazy_static! {
    // 对于 BIOS 而言，会忽略 report_desc
    // 对于标准操作系统而言，会读取 report_desc
    // 因此考虑同时设置两个键盘，操作系统会读取正常键盘的输入，BIOS 则会读取 boot 键盘的输入
    pub static ref KEYBOARD_LEGACY_FHO: hid::FunctionHidOpts = hid::FunctionHidOpts {
        major: 0,
        minor: 0,
        // 设置为 1 后才能在 BIOS 下获取键盘灯的状态
        // https://patchwork.kernel.org/project/linux-usb/patch/20210821134004.363217-1-mdevaev@gmail.com/#24400695
        no_out_endpoint: 1,
        subclass: 1, /* Boot Interface SubClass */
        protocol: 1,  /* Keyboard */
        report_length: 8,
        report_desc: vec![
            // Keyboard
            0x05, 0x01,     /* USAGE_PAGE (Generic Desktop)           */
            0x09, 0x06,     /* USAGE (Keyboard)                       */
            0xa1, 0x01,     /* COLLECTION (Application)               */

            // Padding
            0x75, 0x08,     /*   REPORT_SIZE (8)                      */
            0x95, 0x08,     /*   REPORT_COUNT (8)                     */
            0x81, 0x03,     /*   INPUT (Cnst,Var,Abs)                 */

            // LEDs Output
            0x05, 0x08,     /*   USAGE_PAGE (LEDs)                    */
            0x19, 0x01,     /*   USAGE_MINIMUM (Num Lock)             */
            0x29, 0x05,     /*   USAGE_MAXIMUM (Kana)       */
            0x75, 0x01,     /*   REPORT_SIZE (1)                      */
            0x95, 0x05,     /*   REPORT_COUNT (5)                     */
            0x91, 0x02,     /*   OUTPUT (Data,Var,Abs)                */

            // Output padding
            0x75, 0x03,     /*   REPORT_SIZE (3)                      */
            0x95, 0x01,     /*   REPORT_COUNT (1)                     */
            0x91, 0x03,     /*   OUTPUT (Cnst,Var,Abs)                */

            0xc0            /* END_COLLECTION                         */
        ],
    };

    pub static ref KEYBOARD_FHO: hid::FunctionHidOpts = hid::FunctionHidOpts {
        major: 0,
        minor: 0,
        no_out_endpoint: 1,
        subclass: 1, /* Boot Interface SubClass */
        protocol: 1,  /* Keyboard */
        report_length: 0x22,
        report_desc: vec![
            // Keyboard
            0x05, 0x01,     /* USAGE_PAGE (Generic Desktop)           */
            0x09, 0x06,     /* USAGE (Keyboard)                       */
            0xa1, 0x01,     /* COLLECTION (Application)               */

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



#[derive(Default)]
pub struct Keyboard {
    pub led: [u8; 0x20],
    pub keys: [u8; 0x20],
    pub sys_control_keys: [u8; 0x2],
}

impl Keyboard {
    pub fn clear(&mut self) {
        self.led = [0; 0x20];
        self.keys = [0; 0x20];
        self.sys_control_keys = [0; 2];
    }

    pub fn get_led(&self, led_id: u16) -> bool {
        let idx = led_id as usize / 8;
        if idx < self.led.len() {
            return (self.led[idx] >> (led_id % 8) as u8) & 1 == 1;
        }
        return false;
    }
    pub fn get_key(&self, key_id: u16) -> bool {
        let idx = key_id as usize / 8;
        if idx < self.keys.len() {
            return (self.keys[idx] >> (key_id % 8) as u8) & 1 == 1;
        }
        return false;
    }

    pub fn set_key(&mut self, key_id: u16, status: bool) -> bool {
        let idx = key_id as usize / 8;
        if idx < self.keys.len() {
            let prev = self.keys[idx];
            if status {
                self.keys[idx] |= 1 << (key_id % 8) as u8;
            } else {
                self.keys[idx] &= !(1 << (key_id % 8) as u8);
            }
            return prev != self.keys[idx];
        }
        false
    }

    pub fn get_sys_control_key(&self, sys_control_key_id: u16) -> bool {
        let sys_control_key_id = sys_control_key_id - generic_desktop::usage_id::SYSTEM_POWER_DOWN;
        let idx = sys_control_key_id as usize / 8;
        if idx < self.sys_control_keys.len() {
            return (self.sys_control_keys[idx] >> (sys_control_key_id % 8) as u8) & 1 == 1;
        }
        return false;
    }

    pub fn set_sys_control_key(&mut self, sys_control_key_id: u16, status: bool) -> bool {
        let sys_control_key_id = sys_control_key_id - generic_desktop::usage_id::SYSTEM_POWER_DOWN;
        let idx = sys_control_key_id as usize / 8;
        if idx < self.sys_control_keys.len() {
            let prev = self.sys_control_keys[idx];
            if status {
                self.sys_control_keys[idx] |= 1 << (sys_control_key_id % 8) as u8;
            } else {
                self.sys_control_keys[idx] &= !(1 << (sys_control_key_id % 8) as u8);
            }
            return prev != self.sys_control_keys[idx];
        }
        false
    }


    pub fn get_payload(&self) -> [u8; 0x22] {
        let mut ret = [0; 0x22];
        let (left, right) = ret.split_at_mut(0x20);
        left.copy_from_slice(&self.keys);
        right.copy_from_slice(&self.sys_control_keys);
        ret
    }

    pub fn get_legacy_payload(&self) -> [u8; 0x8] {
        let mut ret = [0_u8; 0x8];
        let mut ctrl_val = 0_u8;
        for i in usage_id::KEYBOARD_LEFT_CONTROL..=usage_id::KEYBOARD_RIGHT_GUI {
            if self.get_key(i) {
                ctrl_val |= 1 << (i - usage_id::KEYBOARD_LEFT_CONTROL);
            }
        }
        ret[0] = ctrl_val;

        let mut current_idx = 2;
        for i in 0..=usage_id::KEYBOARD_APPLICATION {
            if current_idx >= ret.len() {
                break;
            }
            if self.get_key(i) {
                ret[current_idx] = i as u8;
                current_idx += 1;
            }
        }
        ret
    }
}


pub struct KeyboardDevice {
    pub keyboard: Mutex<Keyboard>,
    keyboard_dev_read: Mutex<AsyncFd>,
    keyboard_dev_write: Mutex<AsyncFd>,
    keyboard_legacy_dev_read: Mutex<AsyncFd>,
    keyboard_legacy_dev_write: Mutex<AsyncFd>,
    pub keyboard_update_sender: Sender<[u8; 0x20]>,
}

impl KeyboardDevice {
    // AsyncFd::new must call in tokio async runtime
    pub async fn new(keyboard_minor: i32, keyboard_legacy_minor: i32) -> error::Result<Self> {
        let keyboard_dev_name = format!("/dev/hidg{keyboard_minor}");
        let keyboard_legacy_dev_name = format!("/dev/hidg{keyboard_legacy_minor}");

        let keyboard_dev_read =
            AsyncFd::try_from(fcntl::open(keyboard_dev_name.as_str(), fcntl::OFlag::O_RDONLY, Mode::empty())
                .map_err(|err| error::ErrorKind::io(err.into(), &keyboard_dev_name))?
            ).unwrap();

        let keyboard_dev_write =
            AsyncFd::try_from(fcntl::open(keyboard_dev_name.as_str(), fcntl::OFlag::O_WRONLY, Mode::empty())
                .map_err(|err| error::ErrorKind::io(err.into(), &keyboard_dev_name))?
            ).unwrap();

        let keyboard_legacy_dev_read =
            AsyncFd::try_from(fcntl::open(keyboard_legacy_dev_name.as_str(), fcntl::OFlag::O_RDONLY, Mode::empty())
                .map_err(|err| error::ErrorKind::io(err.into(), &keyboard_legacy_dev_name))?
            ).unwrap();


        let keyboard_legacy_dev_write =
            AsyncFd::try_from(fcntl::open(keyboard_legacy_dev_name.as_str(), fcntl::OFlag::O_WRONLY, Mode::empty())
                .map_err(|err| error::ErrorKind::io(err.into(), &keyboard_legacy_dev_name))?
            ).unwrap();

        let (sender, _) = tokio::sync::watch::channel([0; 0x20]);
        let ret = Self {
            keyboard: Default::default(),
            keyboard_dev_read: Mutex::new(keyboard_dev_read),
            keyboard_dev_write: Mutex::new(keyboard_dev_write),
            keyboard_legacy_dev_read: Mutex::new(keyboard_legacy_dev_read),
            keyboard_legacy_dev_write: Mutex::new(keyboard_legacy_dev_write),
            keyboard_update_sender: sender,
        };

        Ok(ret)
    }

    pub async fn set_key(&self, key_id: u16, status: bool) -> bool {
        return self.keyboard.lock().await
            .set_key(key_id, status);
    }

    pub async fn set_sys_control_key(&self, sys_control_key_id: u16, status: bool) -> bool {
        return self.keyboard.lock().await
            .set_sys_control_key(sys_control_key_id, status);
    }

    pub async fn recv_legacy(&self) -> error::Result<()> {
        let mut led_buf = [0_u8];
        let mut keyboard_legacy_dev_read = self.keyboard_legacy_dev_read.lock().await;
        keyboard_legacy_dev_read.read_exact(&mut led_buf).await
            .map_err(|err| error::ErrorKind::io(err, "keyboard_legacy_dev"))?;
        println!("keyboard_legacy_dev: {led_buf:?}");
        let mut keyboard = self.keyboard.lock().await;
        keyboard.led[0] = (keyboard.led[0] & 0xe0) | (led_buf[0] & 0x1f);

        self.keyboard_update_sender.send_if_modified(|keyboard_state| {
            if keyboard_state[0] != keyboard.led[0] {
                keyboard_state[0] = keyboard.led[0];
                true
            } else {
                false
            }
        });
        Ok(())
    }

    pub async fn recv(&self) -> error::Result<()> {
        let mut led_buf = [0_u8; 0x20];
        let mut keyboard_dev_read = self.keyboard_dev_read.lock().await;
        let read_len = keyboard_dev_read.read(&mut led_buf).await
            .map_err(|err| error::ErrorKind::io(err, "keyboard_dev"))?;
        if read_len != 0x20 {
            println!("keyboard_dev ignore: {:?}", &led_buf[..read_len]);
            return Ok(());
        }
        println!("keyboard_dev: {led_buf:?}");
        let mut keyboard = self.keyboard.lock().await;
        keyboard.led.copy_from_slice(&led_buf);
        self.keyboard_update_sender.send_if_modified(|keyboard_state| {
            let mut ret = false;
            for i in 0..0x20_usize {
                if keyboard_state[i] != keyboard.led[i] {
                    keyboard_state[i] = keyboard.led[i];
                    ret = true;
                }
            }
            ret
        });
        Ok(())
    }

    pub async fn send(&self) -> error::Result<()> {
        let mut keyboard_dev = self.keyboard_dev_write.lock().await;
        let payload = self.keyboard.lock().await.get_payload();
        println!("send {payload:?}");
        keyboard_dev.write_all(&payload).await
            .map_err(|err| error::ErrorKind::io(err, "keyboard_dev"))?;
        Ok(())
    }
    pub async fn send_legacy(&self) -> error::Result<()> {
        let mut keyboard_legacy_dev = self.keyboard_legacy_dev_write.lock().await;
        let payload = self.keyboard.lock().await.get_legacy_payload();
        println!("send_legacy {payload:?}");
        keyboard_legacy_dev.write_all(&payload).await
            .map_err(|err| error::ErrorKind::io(err, "keyboard_legacy_dev"))?;
        Ok(())
    }
}