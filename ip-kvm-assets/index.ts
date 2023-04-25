function set_key(event: KeyboardEvent, status: number) {
    if (!(event.code in keycode_hid_map)) {
        console.log('Not found', event);
        return;
    }
    if (socket.readyState != socket.OPEN) {
        return;
    }
    let hid_code = keycode_hid_map[event.code];
    let buffer = new ArrayBuffer(3);

    let view = new Uint8Array(buffer);
    view[0] = 0;
    view[1] = hid_code;
    view[2] = status;
    socket.send(buffer);
}

function on_key_up(event: KeyboardEvent) {
    if (document.activeElement.id != "stream_url") {
        set_key(event, 0);
        event.preventDefault();
        event.stopPropagation();
    }
}

function on_key_down(event: KeyboardEvent) {
    if (document.activeElement.id != "stream_url") {
        set_key(event, 1);
        event.preventDefault();
        event.stopPropagation();
    }
}


let keycode_hid_map = {
    "AltLeft": 0xe2,
    "AltRight": 0xe6,
    "ArrowDown": 0x51,
    "ArrowLeft": 0x50,
    "ArrowRight": 0x4f,
    "ArrowUp": 0x52,
    "Backquote": 0x35,
    "Backslash": 0x31,
    "Backspace": 0x2a,
    "BracketLeft": 0x2f,
    "BracketRight": 0x30,
    "CapsLock": 0x39,
    "Comma": 0x36,
    "ControlLeft": 0xe0,
    "Delete": 0x4c,
    "Digit0": 0x27,
    "Digit1": 0x1e,
    "Digit2": 0x1f,
    "Digit3": 0x20,
    "Digit4": 0x21,
    "Digit5": 0x22,
    "Digit6": 0x23,
    "Digit7": 0x24,
    "Digit8": 0x25,
    "Digit9": 0x26,
    "End": 0x4d,
    "Enter": 0x28,
    "Equal": 0x2e,
    "Escape": 0x29,
    "F1": 0x3a,
    "F2": 0x3b,
    "F3": 0x3c,
    "F4": 0x3d,
    "F5": 0x3e,
    "F6": 0x3f,
    "F7": 0x40,
    "F8": 0x41,
    "F9": 0x42,
    "F10": 0x43,
    "F11": 0x44,
    "F12": 0x45,
    "Home": 0x4a,
    "IntlBackslash": 0x31,
    "KeyA": 0x04,
    "KeyB": 0x05,
    "KeyC": 0x06,
    "KeyD": 0x07,
    "KeyE": 0x08,
    "KeyF": 0x09,
    "KeyG": 0x0a,
    "KeyH": 0x0b,
    "KeyI": 0x0c,
    "KeyJ": 0x0d,
    "KeyK": 0x0e,
    "KeyL": 0x0f,
    "KeyM": 0x10,
    "KeyN": 0x11,
    "KeyO": 0x12,
    "KeyP": 0x13,
    "KeyQ": 0x14,
    "KeyR": 0x15,
    "KeyS": 0x16,
    "KeyT": 0x17,
    "KeyU": 0x18,
    "KeyV": 0x19,
    "KeyW": 0x1a,
    "KeyX": 0x1b,
    "KeyY": 0x1c,
    "KeyZ": 0x1d,
    "MetaLeft": 0xe3,
    "MetaRight": 0xe7,
    "Minus": 0x2d,
    "NumpadEnter": 0x58,
    "PageDown": 0x4e,
    "PageUp": 0x4b,
    "Period": 0x37,
    "Quote": 0x34,
    "Semicolon": 0x33,
    "ShiftLeft": 0xe1,
    "ShiftRight": 0xe5,
    "Slash": 0x38,
    "Space": 0x2c,
    "Tab": 0x2b
}

let socket = new WebSocket("ws://" + location.host + '/keyboard');
socket.binaryType = "arraybuffer"
socket.onclose = function (event: CloseEvent) {
    if (event.wasClean) {
        alert(`[close] Connection closed cleanly, code=${event.code} reason=${event.reason}`);
    } else {
        // 例如服务器进程被杀死或网络中断
        // 在这种情况下，event.code 通常为 1006
        alert('[close] Connection died');
    }
    socket = new WebSocket("ws://" + location.host + '/keyboard');
};

socket.onmessage = function (event: MessageEvent) {
    let data = new Uint8Array(event.data)
    console.log(data);
}

window.onresize = function (event: UIEvent) {
    resize_video()
}

function resize_video() {
    let img: HTMLImageElement = document.getElementById("video") as HTMLImageElement;
    img.width = window.innerWidth - 25;
    img.height = window.innerHeight - 45;
}

function init_stream_url_input() {
    let stream_url_input = document.getElementById("stream_url") as HTMLInputElement;

    stream_url_input.addEventListener("change", function (event: Event) {
        let img: HTMLImageElement = document.getElementById("video") as HTMLImageElement;
        let stream_url_input = event.target as HTMLInputElement;
        img.src = stream_url_input.value;
        localStorage.setItem('stream_url', JSON.stringify(stream_url_input.value));
    });

    if ('stream_url' in localStorage) {
        stream_url_input.value = JSON.parse(localStorage.getItem('stream_url'));
        let img: HTMLImageElement = document.getElementById("video") as HTMLImageElement;
        img.src = stream_url_input.value;
    }
}


resize_video();

document.addEventListener('keydown', on_key_down);
document.addEventListener('keyup', on_key_up);
init_stream_url_input();
