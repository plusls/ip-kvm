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

    let stream_url = localStorage.getItem('stream_url');
    if (stream_url != null) {
        stream_url_input.value = JSON.parse(stream_url);
        let img: HTMLImageElement = document.getElementById("video") as HTMLImageElement;
        img.src = stream_url_input.value;
    }
}

init_stream_url_input();
resize_video();
