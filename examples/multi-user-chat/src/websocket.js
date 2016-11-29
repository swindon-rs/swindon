import render from './render'

let websocket = null
let timeout = 50
let timeout_token = null
let current_room = null
export var current_room_messages = null
export var metadata = {}
export var state = ""
export var room_list = []
let rooms = {}

let next_request_id = 0
let promises = {}

let insert_history = room => messages => {
    if(current_room !== room) {
        return
    }
    let existing = {}
    for(let msg of current_room_messages) {
        existing[msg.id] = msg
    }
    for(let msg of messages) {
        let old = existing[msg.id]
        if(old) {
            for(var k of Object.keys(msg)) {
                old[k] = msg[k]
            }
        } else {
            current_room_messages.push(msg)
        }
    }
    current_room_messages.sort(function(a, b) {
        return b.id - a.id
    })
}

function open() {
    state = "Connected. Authenticating..."
    render()
    if(timeout_token) {
        clearTimeout(timeout_token)
        timeout_token = null
    }
    if(current_room) {
        call('enter_room', current_room)
        let room = current_room
        call('get_history', room).then(insert_history(room))
    }
}

function close(e) {
    if(timeout_token) {
        clearTimeout(timeout_token);
    } else {
        timeout *= 2
        if(timeout > 300000) {
            timeout = 300000;
        }
    }
    state = ("Broken. Reconnecting in " +
             (timeout/1000).toFixed(0) + " sec...")
    render()
    timeout_token = setTimeout(timeout, reconnect)
    console.error("Websocket closed", e)
}

function reconnect() {
    if(timeout_token) {
        clearTimeout(timeout_token);
        timeout_token = null;
    }
    if(websocket) {
        websocket.onclose = null
        try {
            websocket.close()
        } catch(e) {}
        state = "Reconnecting..."
    } else {
        state = "Connecting..."
    }
    websocket = new WebSocket("ws://" + location.host + "/")
    websocket.onopen = open
    websocket.onclose = close
    websocket.onmessage = message
}

function update_rooms(data) {
    for(let k of Object.keys(data)) {
        let nroom = data[k];
        let r
        if(k in rooms) {
            r = rooms[k]
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
        } else {
            r = nroom
            r.name = k
            rooms[k] = r
            room_list.push(r)
        }
        r.unseen = (r.last_message_counter || 0) -
                   (r.last_seen_counter || 0)
    }
    room_list.sort(function(a, b) {
        return a.name.localeCompare(b.name)
    })
}

function message(ev) {
    var data = JSON.parse(ev.data)

    // Not doing this on connect to be sure not to reconnect too fast
    // when connection doesn't actually work
    timeout = 50

    switch(data[0]) {
        case 'hello':
            metadata = data[2]
            state = "Connected."
            break;
        case 'message':
            console.debug("Message", data)
            if(data[1].topic === 'muc.' + current_room) {
                // TODO(tailhook) deduplicate
                current_room_messages.unshift(data[2])
            }
            break;
        case 'result': {
            let rid = data[1].request_id
            let prom = promises[rid]
            delete promises[rid]
            prom.resolve(data[2])
            break;
        }
        case 'error': {
            let rid = data[1].request_id
            if(rid) {
                let prom = promises[rid]
                delete promises[rid]
                prom.reject(data[2])
            }
            break;
        }
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
    next_request_id += 1
    var prom = new Promise(function(resolve, reject) {
        promises[next_request_id] = {
            resolve: resolve,
            reject: reject,
        }
    })
    websocket.send(JSON.stringify(
        ['muc.' + method, {'request_id': next_request_id}, args, {}]))
    return prom
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
    if(websocket.readyState === WebSocket.OPEN) {
        if(current_room) {
            call('switch_room', current_room, roomName)
        } else {
            call('enter_room', roomName)
        }
    }
    current_room = roomName
    current_room_messages = []
    call('get_history', roomName).then(insert_history(roomName))
}

export function leave_room(route) {
    call('leave_room', current_room)
    current_room = null
    current_room_messages = null
}

export function send_message(text) {
    call('message', current_room, text)
}
