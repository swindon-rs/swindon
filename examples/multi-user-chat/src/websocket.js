import render from './render'

let websocket = null
let timeout = 50
let timeout_token = null
let current_room = null
export var metadata = {}
export var state = ""
export var room_list = []
var rooms = {}

function open() {
    state = "Connected. Authenticating..."
    render()
    if(timeout_token) {
        clearTimeout(timeout_token)
        timeout_token = null
    }
    timeout = 50
    if(current_room) {
        call('enter_room', current_room)
    }
}

function error(e) {
    // TODO(tailhook) update state
    if(timeout_token) {
        clearTimeout(timeout_token);
    } else {
        timeout *= 2
        if(timeout > 300) {
            timeout = 300;
        }
    }
    state = ("Broken. Reconnecting in " +
             (timeout/1000).toFixed(0) + " sec...")
    render()
    timeout_token = setTimeout(timeout, reconnect)
    console.error("Websocket error", e)
}

function reconnect(e) {
    if(e) {
        console.error("Websocket close", e)
    }
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

function update_rooms(data) {
    for(var k of Object.keys(data)) {
        var nroom = data[k];
        if(k in rooms) {
            let r = rooms[k]
            if(!r.last_message_counter ||
                    r.last_message_counter < nroom.last_message_counter)
            {
                r.last_message_counter = nroom.last_message_counter
            }
            if(!r.last_seen_counter ||
                    r.last_seen_counter < nroom.last_seen_counter)
            {
                r.last_seen_counter = nroom.last_seen_counter
            }
            r.unseen = nroom.last_message_counter - nroom.last_seen_counter
        } else {
            let r = nroom
            r.name = k
            r.unseen = nroom.last_message_counter - nroom.last_seen_counter
            rooms[k] = r
            room_list.push(r)
        }
    }
    room_list.sort(function(a, b) {
        return a.name.localeCompare(b.name)
    })
    console.log("rooms", room_list)
}

function message(ev) {
    var data = JSON.parse(ev.data)
    switch(data[0]) {
        case 'hello':
            metadata = data[2]
            state = "Connected."
            break;
        case 'message':
            console.debug("Message", data)
            break;
        case 'lattice':
            console.debug("Lattice", data)
            switch(data[1].namespace) {
                case 'muc':
                    update_rooms(data[2]);
                    break;
                default:
                    console.error("Lattice", data)
                    break;
            }
            break;
        default:
            console.error("Skipping message", data)
    }
    render()
}

function call(method, ...args) {
    websocket.send(JSON.stringify(
        ['muc.' + method, {'request_id': 0}, args, {}]))
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


export function enter_room(route) {
    let { params: {roomName}} = route;
    console.log("ENTER", roomName, websocket.readyState)
    if(websocket.readyState === WebSocket.OPEN) {
        if(current_room) {
            call('switch_room', current_room, roomName)
        } else {
            call('enter_room', roomName)
        }
    }
    current_room = roomName
}

export function leave_room(route) {
    call('leave_room', current_room)
    current_room = null
}
