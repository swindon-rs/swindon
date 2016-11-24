import render from './render'

let websocket = null
let timeout = 50
let timeout_token = null
export var metadata = {}
export var state = ""

function open() {
    state = "Connected. Authenticating..."
    render()
    if(timeout_token) {
        clearTimeout(timeout_token)
        timeout_token = null
    }
    timeout = 50
}

function error() {
    // TODO(tailhook) update state
    timeout *= 2
    state = ("Broken. Reconnecting in " +
             (timeout/1000).toFixed(0) + " sec...")
    render()
    if(timeout_token) clearTimeout(timeout_token);
    timeout_token = setTimeout(timeout, reconnect)
}

function reconnect() {
    if(websocket) {
        websocket.onclose = null
        websocket.onerror = null
        try {
            websocket.close()
        } catch(e) {}
        state = "Reconnecting..."
    } else {
        state = "Connecting..."
    }
    websocket = new WebSocket("ws://" + location.host + "/")
    websocket.onopen = open
    websocket.onclose = reconnect
    websocket.onerror = error
    websocket.onmessage = message
}

function message(ev) {
    var data = JSON.parse(ev.data)
    switch(data[0]) {
        case 'hello':
            metadata = data[2]
            break;
        case 'message':
            console.debug("Message", data)
            break;
        case 'lattice':
            console.debug("Lattice", data)
            break;
        default:
            console.error("Skipping message", data)
    }
    render()
}

export function start() {
    if(websocket) {
        return
    }
    reconnect()
}

export function stop() {
    websocket.close();
    if(timeout_token) {
        clearTimeout(timeout_token);
        timeout_token = null;
    }
    state = "Stopped."
}
