pub mod usage_id {
    pub const POINTER: u16 = 0x1;
    pub const MOUSE: u16 = 0x2;
    pub const JOYSTICK: u16 = 0x4;
    pub const GAMEPAD: u16 = 0x5;
    pub const KEYBOARD: u16 = 0x6;
    pub const KEYPAD: u16 = 0x7;
    pub const MULTI_AXIS_CONTROLLER: u16 = 0x8;
    pub const TABLET_PC_SYSTEM_CONTROLS: u16 = 0x9;
    pub const WATER_COOLING_DEVICE: u16 = 0xa;
    pub const COMPUTER_CHASSIS_DEVICE: u16 = 0xb;
    pub const WIRELESS_RADIO_CONTROLS: u16 = 0xc;
    pub const PORTABLE_DEVICE_CONTROL: u16 = 0xd;
    pub const SYSTEM_MULTI_AXIS_CONTROLLER: u16 = 0xe;
    pub const SPATIAL_CONTROLLER: u16 = 0xf;
    pub const ASSISTIVE_CONTROL: u16 = 0x10;
    pub const DEVICE_DOCK: u16 = 0x11;
    pub const DOCKABLE_DEVICE: u16 = 0x12;
    pub const CALL_STATE_MANAGEMENT_CONTROL: u16 = 0x13;
    pub const X: u16 = 0x30;
    pub const Y: u16 = 0x31;
    pub const Z: u16 = 0x32;
    pub const RX: u16 = 0x33;
    pub const RY: u16 = 0x34;
    pub const RZ: u16 = 0x35;
    pub const SLIDER: u16 = 0x36;
    pub const DIAL: u16 = 0x37;
    pub const WHEEL: u16 = 0x38;
    pub const HAT_SWITCH: u16 = 0x39;
    pub const COUNTED_BUFFER: u16 = 0x3a;
    pub const BYTE_COUNT: u16 = 0x3b;
    pub const MOTION_WAKEUP: u16 = 0x3c;
    pub const START: u16 = 0x3d;
    pub const SELECT: u16 = 0x3e;
    pub const VX: u16 = 0x40;
    pub const VY_DV: u16 = 0x41;
    pub const VZ: u16 = 0x42;
    pub const VBRX: u16 = 0x43;
    pub const VBRY: u16 = 0x44;
    pub const VBRZ: u16 = 0x45;
    pub const VNO: u16 = 0x46;
    pub const FEATURE_NOTIFICATION: u16 = 0x47;
    pub const RESOLUTION_MULTIPLIER: u16 = 0x48;
    pub const QX: u16 = 0x49;
    pub const QY: u16 = 0x4a;
    pub const QZ: u16 = 0x4b;
    pub const QW: u16 = 0x4c;
    pub const SYSTEM_CONTROL: u16 = 0x80;
    pub const SYSTEM_POWER_DOWN: u16 = 0x81;
    pub const SYSTEM_SLEEP: u16 = 0x82;
    pub const SYSTEM_WAKE_UP: u16 = 0x83;
    pub const SYSTEM_CONTEXT_MENU: u16 = 0x84;
    pub const SYSTEM_MAIN_MENU: u16 = 0x85;
    pub const SYSTEM_APP_MENU: u16 = 0x86;
    pub const SYSTEM_MENU_HELP: u16 = 0x87;
    pub const SYSTEM_MENU_EXIT: u16 = 0x88;
    pub const SYSTEM_MENU_SELECT: u16 = 0x89;
    pub const SYSTEM_MENU_RIGHT: u16 = 0x8a;
    pub const SYSTEM_MENU_LEFT: u16 = 0x8b;
    pub const SYSTEM_MENU_UP: u16 = 0x8c;
    pub const SYSTEM_MENU_DOWN: u16 = 0x8d;
    pub const SYSTEM_COLD_RESTART: u16 = 0x8e;
    pub const SYSTEM_WARM_RESTART: u16 = 0x8f;
    pub const D_PAD_UP: u16 = 0x90;
    pub const D_PAD_DOWN: u16 = 0x91;
    pub const D_PAD_RIGHT: u16 = 0x92;
    pub const D_PAD_LEFT: u16 = 0x93;
    pub const INDEX_TRIGGER: u16 = 0x94;
    pub const PALM_TRIGGER: u16 = 0x95;
    pub const THUMBSTICK: u16 = 0x96;
    pub const SYSTEM_FUNCTION_SHIFT: u16 = 0x97;
    pub const SYSTEM_FUNCTION_SHIFT_LOCK: u16 = 0x98;
    pub const SYSTEM_FUNCTION_SHIFT_LOCK_INDICATOR: u16 = 0x99;
    pub const SYSTEM_DISMISS_NOTIFICATION: u16 = 0x9a;
    pub const SYSTEM_DO_NOT_DISTURB: u16 = 0x9b;
    pub const SYSTEM_DOCK: u16 = 0xa0;
    pub const SYSTEM_UNDOCK: u16 = 0xa1;
    pub const SYSTEM_SETUP: u16 = 0xa2;
    pub const SYSTEM_BREAK: u16 = 0xa3;
    pub const SYSTEM_DEBUGGER_BREAK: u16 = 0xa4;
    pub const APPLICATION_BREAK: u16 = 0xa5;
    pub const APPLICATION_DEBUGGER_BREAK: u16 = 0xa6;
    pub const SYSTEM_SPEAKER_MUTE: u16 = 0xa7;
    pub const SYSTEM_HIBERNATE: u16 = 0xa8;
    pub const SYSTEM_MICROPHONE_MUTE: u16 = 0xa9;
    pub const SYSTEM_DISPLAY_INVERT: u16 = 0xb0;
    pub const SYSTEM_DISPLAY_INTERNAL: u16 = 0xb1;
    pub const SYSTEM_DISPLAY_EXTERNAL: u16 = 0xb2;
    pub const SYSTEM_DISPLAY_BOTH: u16 = 0xb3;
    pub const SYSTEM_DISPLAY_DUAL: u16 = 0xb4;
    pub const SYSTEM_DISPLAY_TOGGLE_INT_OR_EXT_MODE: u16 = 0xb5;
    pub const SYSTEM_DISPLAY_SWAP_PRIMARY_OR_SECONDARY: u16 = 0xb6;
    pub const SYSTEM_DISPLAY_TOGGLE_LCD_AUTOSCALE: u16 = 0xb7;
    pub const SENSOR_ZONE: u16 = 0xc0;
    pub const RPM: u16 = 0xc1;
    pub const COOLANT_LEVEL: u16 = 0xc2;
    pub const COOLANT_CRITICAL_LEVEL: u16 = 0xc3;
    pub const COOLANT_PUMP: u16 = 0xc4;
    pub const CHASSIS_ENCLOSURE: u16 = 0xc5;
    pub const WIRELESS_RADIO_BUTTON: u16 = 0xc6;
    pub const WIRELESS_RADIO_LED: u16 = 0xc7;
    pub const WIRELESS_RADIO_SLIDER_SWITCH: u16 = 0xc8;
    pub const SYSTEM_DISPLAY_ROTATION_LOCK_BUTTON: u16 = 0xc9;
    pub const SYSTEM_DISPLAY_ROTATION_LOCK_SLIDER_SWITCH: u16 = 0xca;
    pub const CONTROL_ENABLE: u16 = 0xcb;
    pub const DOCKABLE_DEVICE_UNIQUE_ID: u16 = 0xd0;
    pub const DOCKABLE_DEVICE_VENDOR_ID: u16 = 0xd1;
    pub const DOCKABLE_DEVICE_PRIMARY_USAGE_PAGE: u16 = 0xd2;
    pub const DOCKABLE_DEVICE_PRIMARY_USAGE_ID: u16 = 0xd3;
    pub const DOCKABLE_DEVICE_DOCKING_STATE: u16 = 0xd4;
    pub const DOCKABLE_DEVICE_DISPLAY_OCCLUSION: u16 = 0xd5;
    pub const DOCKABLE_DEVICE_OBJECT_TYPE: u16 = 0xd6;
    pub const CALL_ACTIVE_LED: u16 = 0xe0;
    pub const CALL_MUTE_TOGGLE: u16 = 0xe1;
    pub const CALL_MUTE_LED: u16 = 0xe2;
}