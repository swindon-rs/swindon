import render from './render'
import {Swindon, Lattice} from 'swindon'

let swindon = null
let roster = new Lattice({onUpdate: update_rooms})
let current_room = null
let room_guard = null
let status_interval = null
let render_scheduled = false
export var current_room_messages = null
export var metadata = {}
export var state = ""
export var room_list = []
let rooms = {}

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
    schedule_render()
}

function update_rooms(updated_keys) {
    for(let k of updated_keys) {
        let r
        if(k in rooms) {
            r = rooms[k]
        } else {
            r = {}
            r.name = k
            rooms[k] = r
            room_list.push(r)
        }
        r.unseen = roster.getCounter(k, 'last_message') -
            roster.getCounter(k, 'last_seen')
    }
    room_list.sort(function(a, b) {
        return a.name.localeCompare(b.name)
    })
    schedule_render()
}

function new_message(msg) {
    current_room_messages.unshift(msg)
    schedule_render()
}

export function start() {
    if(swindon) {
        return
    }
    swindon = new Swindon("ws://" + location.host + "/", {
        onStateChange: update_status,
    })
    // roster lattice is automatically subscribed to on server start
    swindon.guard().lattice("muc", "", roster)
}

export function stop() {
    console.error("Closing", swindon)
    swindon.close();
    swindon = null
    state = "Stopped."
    schedule_render()
}


export function enter_room(route) {
    let { params: {roomName}} = route;
    if(room_guard) {
        room_guard.close();
    }
    current_room_messages = []
    current_room = roomName
    room_guard = swindon.guard()
        .init("muc.get_history", [roomName], {}, insert_history(roomName))
        .init("muc.enter_room", [roomName])
        .listen("muc." + roomName, new_message)
        .deinit("muc.leave_room", [roomName])
}

export function leave_room(route) {
    if(room_guard) {
        room_guard.close();
    }
    room_guard = null
    current_room = null
    current_room_messages = null
}

export function send_message(text) {
    swindon.call('muc.message', [current_room, text])
}

function update_status(state) {
    if(status_interval) {
        clearInterval(status_interval)
        status_interval = null;
    }
    console.log("Websocket status changed", state)
    switch(state.status) {
        case "wait":
            let left = Math.round((state.reconnect_time
                                   - Date.now())/1000);
            if(left < 1) {
                set_status("Reconnecting...")
            } else {
                status_interval = setInterval(
                    _ => update_status(swindon.state()),
                    1000)
                set_status("Reconnecting in " + left + " seconds")
            }
            break;
        case "active":
            set_status("Connected");
            break;
        case "connecting":
            set_status("Connecting...")
            break;
        default:
            // it's "closed" or maybe some future value
            set_status("No connection.");
            break;
    }
}

function set_status(s) {
    state = s;
    schedule_render()
}

function schedule_render() {
    if(!render_scheduled) {
        render_scheduled = true
        requestAnimationFrame(_ => {
            render_scheduled = false
            render()
        })
    }
}
