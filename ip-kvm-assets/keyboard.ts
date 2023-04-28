function set_key(event: KeyboardEvent, status: number) {
    let hid_code = keycode_hid_map.get(event.code);
    if (hid_code == null) {
        console.log('Not found', event);
        return;
    }
    send_key(hid_code, status);
}

function send_key(hid_code: number, status: number) {
    if (keyboard_socket == null || keyboard_socket.readyState != WebSocket.OPEN) {
        return;
    }
    let buffer = new ArrayBuffer(3);
    let view = new Uint8Array(buffer);
    view[0] = 0;
    view[1] = hid_code;
    view[2] = status;
    keyboard_socket.send(buffer);
}

function on_key_up(event: KeyboardEvent) {
    if (document.activeElement == null || (document.activeElement.id != "stream_url" && document.activeElement.id != "paste_input")) {
        set_key(event, 0);
        event.preventDefault();
        event.stopPropagation();
    }
}

function on_key_down(event: KeyboardEvent) {
    if (document.activeElement == null || (document.activeElement.id != "stream_url" && document.activeElement.id != "paste_input")) {
        set_key(event, 1);
        event.preventDefault();
        event.stopPropagation();
    }
}

let ascii_hid_map: Map<string, number> = (() => {


    let ret: Map<string, number> = new Map([
        ["\t", 0x2b],
        ["\n", 0x28],
        ["\r", 0x28],
        [" ", 0x2c],
        ["!", -0x1e],
        ["\"", -0x34],
        ["#", -0x20],
        ["$", -0x21],
        ["%", -0x22],
        ["&", -0x24],
        ["'", 0x34],
        ["(", -0x26],
        [")", -0x27],
        ["*", -0x25],
        ["+", -0x2e],
        [",", 0x36],
        ["-", 0x2d],
        [".", 0x37],
        ["/", 0x38],
        // 0-9 auto generate
        [":", -0x33],
        [";", 0x33],
        ["<", -0x36],
        ["=", 0x2e],
        [">", -0x37],
        ["?", -0x38],
        ["@", -0x1f],
        // A-Z auto generate
        ["[", 0x2f],
        ["\\", 0x31],
        ["]", 0x30],
        ["^", -0x23],
        ["_", -0x2d],
        ["`", 0x35],
        // a-z auto generate
        ["{", -0x2f],
        ["|", -0x31],
        ["}", -0x30],
        ["~", -0x35],
    ]);
    for (let i = 0; i < 26; ++i) {
        ret.set(String.fromCharCode(i + 'a'.charCodeAt(0)), 0x04 + i);
        ret.set(String.fromCharCode(i + 'A'.charCodeAt(0)), -(0x04 + i));
    }

    ret.set('0', 0x27);
    for (let i = 0; i < 9; ++i) {
        ret.set(String.fromCharCode(i + '1'.charCodeAt(0)), 0x1e + i);
    }

    return ret;
})();

function send_ascii_str(s: string): boolean {
    s = s.replaceAll("\r\n", "\n");
    for (let i = 0; i < s.length; ++i) {
        if (ascii_hid_map.get(s.charAt(i)) == null) {
            return false;
        }
    }
    for (let i = 0; i < s.length; ++i) {
        let ch = s.charAt(i);
        let hid_code = ascii_hid_map.get(ch) as number;
        if (ch != '\n' && ch != '\r' && ch != '\t' && ch != ' ') {
            let shift = hid_code < 0;
            if (((key_board_status[0] & 4) != 0) && ((ch.charCodeAt(0) >= 'a'.charCodeAt(0) && ch.charCodeAt(0) <= 'z'.charCodeAt(0)) ||
                (ch.charCodeAt(0) >= 'A'.charCodeAt(0) && ch.charCodeAt(0) <= 'Z'.charCodeAt(0)))) {
                shift = !shift;
            }
            hid_code = Math.abs(hid_code);
            if (shift) {
                send_key(0xe1, 1);
            }
            send_key(hid_code, 1);
            send_key(hid_code, 0);
            if (shift) {
                send_key(0xe1, 0);
            }
        } else {
            send_key(hid_code, 1);
            send_key(hid_code, 0);
        }
    }
    return true;
}

let keycode_hid_map: Map<string, number> = new Map([
    ["AltLeft", 0xe2],
    ["AltRight", 0xe6],
    ["ArrowDown", 0x51],
    ["ArrowLeft", 0x50],
    ["ArrowRight", 0x4f],
    ["ArrowUp", 0x52],
    ["Backquote", 0x35],
    ["Backslash", 0x31],
    ["Backspace", 0x2a],
    ["BracketLeft", 0x2f],
    ["BracketRight", 0x30],
    ["CapsLock", 0x39],
    ["Comma", 0x36],
    ["ControlLeft", 0xe0],
    ["ControlRight", 0xe4],
    ["Delete", 0x4c],
    ["Digit0", 0x27],
    ["Digit1", 0x1e],
    ["Digit2", 0x1f],
    ["Digit3", 0x20],
    ["Digit4", 0x21],
    ["Digit5", 0x22],
    ["Digit6", 0x23],
    ["Digit7", 0x24],
    ["Digit8", 0x25],
    ["Digit9", 0x26],
    ["End", 0x4d],
    ["Enter", 0x28],
    ["Equal", 0x2e],
    ["Escape", 0x29],
    ["F1", 0x3a],
    ["F2", 0x3b],
    ["F3", 0x3c],
    ["F4", 0x3d],
    ["F5", 0x3e],
    ["F6", 0x3f],
    ["F7", 0x40],
    ["F8", 0x41],
    ["F9", 0x42],
    ["F10", 0x43],
    ["F11", 0x44],
    ["F12", 0x45],
    ["Home", 0x4a],
    ["IntlBackslash", 0x31],
    ["KeyA", 0x04],
    ["KeyB", 0x05],
    ["KeyC", 0x06],
    ["KeyD", 0x07],
    ["KeyE", 0x08],
    ["KeyF", 0x09],
    ["KeyG", 0x0a],
    ["KeyH", 0x0b],
    ["KeyI", 0x0c],
    ["KeyJ", 0x0d],
    ["KeyK", 0x0e],
    ["KeyL", 0x0f],
    ["KeyM", 0x10],
    ["KeyN", 0x11],
    ["KeyO", 0x12],
    ["KeyP", 0x13],
    ["KeyQ", 0x14],
    ["KeyR", 0x15],
    ["KeyS", 0x16],
    ["KeyT", 0x17],
    ["KeyU", 0x18],
    ["KeyV", 0x19],
    ["KeyW", 0x1a],
    ["KeyX", 0x1b],
    ["KeyY", 0x1c],
    ["KeyZ", 0x1d],
    ["MetaLeft", 0xe3],
    ["MetaRight", 0xe7],
    ["Minus", 0x2d],
    ["NumpadEnter", 0x58],
    ["PageDown", 0x4e],
    ["PageUp", 0x4b],
    ["Period", 0x37],
    ["Quote", 0x34],
    ["Semicolon", 0x33],
    ["ShiftLeft", 0xe1],
    ["ShiftRight", 0xe5],
    ["Slash", 0x38],
    ["Space", 0x2c],
    ["Tab", 0x2b],
    ["PrintScreen", 0x46],
    ["ScrollLock", 0x47],
    ["Pause", 0x48],
    ["Insert", 0x49],

]);

let key_board_status: Uint8Array = new Uint8Array(new Array(0x20).fill(0));

let keyboard_socket: WebSocket | null = null;

function init_ws() {
    keyboard_socket = new WebSocket("ws://" + location.host + '/keyboard');
    keyboard_socket.binaryType = "arraybuffer"
    keyboard_socket.onclose = function (event: CloseEvent) {
        if (event.wasClean) {
            alert(`[close] Connection closed cleanly, code=${event.code} reason=${event.reason}`);
        } else {
            // 例如服务器进程被杀死或网络中断
            // 在这种情况下，event.code 通常为 1006
            alert('[close] Connection died');
        }
        init_ws();
    };
    keyboard_socket.onmessage = function (event: MessageEvent) {
        key_board_status = new Uint8Array(event.data)
        console.log("recv!", key_board_status);
    }
}

init_ws();

document.addEventListener('keydown', on_key_down);
document.addEventListener('keyup', on_key_up);
