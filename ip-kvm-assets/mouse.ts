let mouse_socket = new WebSocket("ws://" + location.host + '/mouse');
mouse_socket.onclose = function (event: CloseEvent) {
    if (event.wasClean) {
        alert(`[close] Connection closed cleanly, code=${event.code} reason=${event.reason}`);
    } else {
        // 例如服务器进程被杀死或网络中断
        // 在这种情况下，event.code 通常为 1006
        alert('[close] Connection died');
    }
    mouse_socket = new WebSocket("ws://" + location.host + '/mouse');
};


function init_mouse() {
    let img: HTMLImageElement = document.getElementById("video") as HTMLImageElement;
    img.addEventListener("mousedown", function (event: MouseEvent) {
        event.preventDefault();
        event.stopPropagation();
        let pos = translate_pos(event.offsetX, event.offsetY, img.width, img.height);
        send_mouse_data(event.buttons, pos[0], pos[1], 0);
    });
    img.addEventListener("mouseup", function (event: MouseEvent) {
        event.preventDefault();
        event.stopPropagation();
        let pos = translate_pos(event.offsetX, event.offsetY, img.width, img.height);
        send_mouse_data(event.buttons, pos[0], pos[1], 0);
    });
    img.addEventListener("contextmenu", function (event: MouseEvent) {
        // 防止弹出右键菜单
        event.preventDefault();
        event.stopPropagation();
    })

    img.addEventListener("wheel", function (event: WheelEvent) {
        event.preventDefault();
        event.stopPropagation();
        let pos = translate_pos(event.offsetX, event.offsetY, img.width, img.height);
        let wheel = -event.deltaY;
        send_mouse_data(event.buttons, pos[0], pos[1], wheel / 80);
    });
    let i = 0;
    img.addEventListener("mousemove", function (event: MouseEvent) {
        i += 1;
        if (i % 1 == 0) {
            let pos = translate_pos(event.offsetX, event.offsetY, img.width, img.height);
            send_mouse_data(event.buttons, pos[0], pos[1], 0);
            i = 0;
        }

    });
}

function translate_pos(x: number, y: number, width: number, height: number): [number, number] {
    x = Math.round((x / width) * 0x7fff);
    y = Math.round((y / height) * 0x7fff);
    if (x > 0x7fff) {
        x = 0x7fff;
    }
    if (x < 0) {
        x = 0;
    }
    if (y > 0x7fff) {
        y = 0x7fff;
    }
    if (y < 0) {
        y = 0;
    }
    return [x, y];
}

function send_mouse_data(button: number, x: number, y: number, wheel: number) {
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

init_mouse()