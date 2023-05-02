let mouse_socket: WebSocket | null = null;
let mouse_legacy_socket: WebSocket | null = null;

function init_mouse_ws() {
    mouse_socket = new WebSocket("ws://" + location.host + '/v1/ws/mouse');
    mouse_socket.onclose = function (event: CloseEvent) {
        if (event.wasClean) {
            alert(`[close] Connection closed cleanly, code=${event.code} reason=${event.reason}`);
        } else {
            // 例如服务器进程被杀死或网络中断
            // 在这种情况下，event.code 通常为 1006
            alert('[close] Connection died');
        }
        init_mouse_ws();
    };
}

function init_mouse_legacy_ws() {
    mouse_legacy_socket = new WebSocket("ws://" + location.host + '/v1/ws/mouse_legacy');
    mouse_legacy_socket.onclose = function (event: CloseEvent) {
        if (event.wasClean) {
            alert(`[close] Connection closed cleanly, code=${event.code} reason=${event.reason}`);
        } else {
            // 例如服务器进程被杀死或网络中断
            // 在这种情况下，event.code 通常为 1006
            alert('[close] Connection died');
        }
        init_mouse_legacy_ws();
    };
}


function init_mouse() {
    init_mouse_ws();
    init_mouse_legacy_ws();

    let img: HTMLImageElement = document.getElementById("video") as HTMLImageElement;

    let rel_resize = 500;

    let buttons = 0;
    let prev_left_click_start_pos: Array<number> | null = null;
    let prev_left_click_pos: Array<number> | null = null;
    let prev_left_click_time = new Date();

    function mouse_left_button_up(event: MouseEvent) {
        let pos = translate_pos(event.offsetX, event.offsetY, img.width, img.height, rel_resize);
        if (prev_left_click_start_pos != null && pos[0] == prev_left_click_start_pos[0] && pos[1] == prev_left_click_start_pos[1]) {
            send_mouse_legacy_data(buttons | 1, 0, 0, 0);
        }
        prev_left_click_pos = null;
        prev_left_click_start_pos = null;
    }

    function mouse_left_button_down(event: MouseEvent) {
        prev_left_click_pos = translate_pos(event.offsetX, event.offsetY, img.width, img.height, rel_resize);
        prev_left_click_start_pos = prev_left_click_pos;
        let current_time = new Date();
        // 300ms 内双击
        if (current_time.getTime() - prev_left_click_time.getTime() <= 300) {
            buttons |= 1;
        }
        prev_left_click_time = current_time;
    }

    function check_mouse_left_button(event: MouseEvent) {
        if (prev_left_click_start_pos != null && (event.buttons & 1) == 0) {
            buttons = event.buttons;
            mouse_left_button_up(event);
        } else if (prev_left_click_start_pos == null && (event.buttons & 1) == 1) {
            buttons = event.buttons & 6;
            mouse_left_button_down(event);
        } else if ((buttons & 1) == 0 && (event.buttons & 1) == 1) {
            buttons = event.buttons & 6;
        } else {
            buttons = event.buttons;
        }
    }

    img.addEventListener("mousedown", function (event: MouseEvent) {
        event.preventDefault();
        event.stopPropagation();
        let button = document.getElementById("mouse_mode_button") as HTMLButtonElement;
        if (button.textContent == 'true') {
            let pos = translate_pos(event.offsetX, event.offsetY, img.width, img.height, 0x7fff);
            send_mouse_data(event.buttons, pos[0], pos[1], 0);
        } else {
            check_mouse_left_button(event);
            if ((buttons & 1) == 0 || event.button != 1) {
                send_mouse_legacy_data(buttons, 0, 0, 0);
            }
        }
    });
    img.addEventListener("mouseup", function (event: MouseEvent) {
        event.preventDefault();
        event.stopPropagation();
        let button = document.getElementById("mouse_mode_button") as HTMLButtonElement;
        if (button.textContent == 'true') {
            let pos = translate_pos(event.offsetX, event.offsetY, img.width, img.height, 0x7fff);
            send_mouse_data(event.buttons, pos[0], pos[1], 0);
        } else {
            check_mouse_left_button(event);
            send_mouse_legacy_data(buttons, 0, 0, 0);
        }

    });
    img.addEventListener("contextmenu", function (event: MouseEvent) {
        // 防止弹出右键菜单
        event.preventDefault();
        event.stopPropagation();
    })

    img.addEventListener("wheel", function (event: WheelEvent) {
        event.preventDefault();
        event.stopPropagation();
        let button = document.getElementById("mouse_mode_button") as HTMLButtonElement;
        let wheel = -event.deltaY;
        if (button.textContent == 'true') {
            let pos = translate_pos(event.offsetX, event.offsetY, img.width, img.height, 0x7fff);
            send_mouse_data(event.buttons, pos[0], pos[1], wheel / 80);
        } else {
            check_mouse_left_button(event);
            send_mouse_legacy_data(buttons, 0, 0, wheel / 80);
        }
    });
    img.addEventListener("mousemove", function (event: MouseEvent) {
        let button = document.getElementById("mouse_mode_button") as HTMLButtonElement;
        if (button.textContent == 'true') {
            let pos = translate_pos(event.offsetX, event.offsetY, img.width, img.height, 0x7fff);
            send_mouse_data(event.buttons, pos[0], pos[1], 0);
        } else {
            check_mouse_left_button(event);

            if (prev_left_click_pos == null) {
                return;
            }
            let pos = translate_pos(event.offsetX, event.offsetY, img.width, img.height, rel_resize);
            let offset = [pos[0] - prev_left_click_pos[0], pos[1] - prev_left_click_pos[1]];
            prev_left_click_pos = pos;
            send_mouse_legacy_data(buttons, offset[0], offset[1], 0);
        }
    });
    let touch_button = 0;
    let touch_type = 0;
    let prev_touch_start_pos: Array<number> | null = null;
    let prev_touch_pos: Array<number> | null = null;
    let prev_touch_time = new Date();

    function get_touch_type(event: TouchEvent): number {
        if (event.targetTouches.length == 1) {
            return 1;
        } else if (event.targetTouches.length == 2) {
            return 2;
        } else {
            return 0;
        }
    }

    img.addEventListener("touchstart", function (event: TouchEvent) {
        let rect = (event.target as HTMLImageElement).getBoundingClientRect();
        prev_touch_pos = translate_pos(event.targetTouches[0].pageX - rect.left, event.targetTouches[0].pageY - rect.top,
            img.width, img.height, rel_resize);
        prev_touch_start_pos = prev_touch_pos
        touch_type = get_touch_type(event)
        let current_time = new Date();
        // 300ms 内双击
        if (current_time.getTime() - prev_touch_time.getTime() <= 300) {
            touch_button = touch_type;
        }
        prev_touch_time = current_time;
        event.preventDefault();
        event.stopPropagation();
    });
    img.addEventListener("touchend", function (event: TouchEvent) {
        if (prev_touch_start_pos != null && prev_touch_pos != null &&
            prev_touch_start_pos[0] == prev_touch_pos[0] && prev_touch_start_pos[1] == prev_touch_pos[1]) {
            send_mouse_legacy_data(touch_type, 0, 0, 0);
        }
        prev_touch_pos = null;
        prev_touch_start_pos = null;
        touch_button = 0;
        touch_type = 0;
        send_mouse_legacy_data(touch_button, 0, 0, 0);
        event.preventDefault();
        event.stopPropagation();
    });

    img.addEventListener("touchmove", function (event: TouchEvent) {
        if (prev_touch_pos == null) {
            return;
        }
        let rect = img.getBoundingClientRect();
        let pos = translate_pos(event.targetTouches[0].pageX - rect.left, event.targetTouches[0].pageY - rect.top,
            img.width, img.height, rel_resize);
        let offset = [pos[0] - prev_touch_pos[0], pos[1] - prev_touch_pos[1]];
        send_mouse_legacy_data(touch_button, offset[0], offset[1], 0);
        prev_touch_pos = pos;

        event.preventDefault();
        event.stopPropagation();
    });
}


function translate_pos(x: number, y: number, width: number, height: number, resize: number): [number, number] {
    x = Math.round((x / width) * resize);
    y = Math.round((y / height) * resize);
    if (x > resize) {
        x = resize;
    }
    if (x < 0) {
        x = 0;
    }
    if (y > resize) {
        y = resize;
    }
    if (y < 0) {
        y = 0;
    }

    return [x, y];
}

function send_mouse_legacy_data(button: number, x: number, y: number, wheel: number) {
    if (mouse_legacy_socket == null) {
        return;
    }
    if (wheel > 127) {
        wheel = 127;
    } else if (wheel < -127) {
        wheel = -127;
    }
    let buffer = new ArrayBuffer(4);
    let view = new Uint8Array(buffer);
    view[0] = button & 0xff;
    view[1] = x & 0xff;
    view[2] = y & 0xff;
    view[3] = wheel;
    mouse_legacy_socket.send(buffer);
}

function send_mouse_data(button: number, x: number, y: number, wheel: number) {
    if (mouse_socket == null) {
        return;
    }
    if (wheel > 127) {
        wheel = 127;
    } else if (wheel < -127) {
        wheel = -127;
    }
    let buffer = new ArrayBuffer(6);
    let view = new Uint8Array(buffer);
    view[0] = button & 0xff;
    view[1] = x & 0xff;
    view[2] = (x >> 8) & 0x7f;
    view[3] = y & 0xff;
    view[4] = (y >> 8) & 0x7f;
    view[5] = wheel;
    mouse_socket.send(buffer);
}

function mouse_mode_button_on_click() {
    let button = document.getElementById("mouse_mode_button") as HTMLButtonElement;
    if (button.textContent == "true") {
        button.textContent = "false";
    } else {
        button.textContent = "true";
    }
}

init_mouse()