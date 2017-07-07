import { Swindon } from 'swindon'

var swindon = new Swindon("ws://"+location.host, {
    onStateChange: update_status,
})
var mb = document.getElementById('mb');
var status_el = document.getElementById('status_text');
var input = document.getElementById('input');
var my_user_id = null;

input.onkeydown = function(ev) {
    if(ev.which == 13) {
        swindon.call(
            "message",     // method
            [input.value], // args
            {},            // kwargs
        )
        input.value = ''
    }
}

start_chat()
// interval helps to update counter of seconds to reconnect
setInterval(() => update_status(swindon.state()), 1000)

async function start_chat() {
    swindon.guard().listen("message-board", add_message)
    let user_info = await swindon.waitConnected()
    log('info', "Your name is " + user_info['username'])
    input.style.visibility = 'visible'
    input.focus()
}

function update_status(state) {
    console.log("Websocket status changed", state)
    switch(state.status) {
        case "wait":
            let left = Math.round((state.reconnect_time  - Date.now())/1000);
            if(left < 1) {
                status_el.textContent = "Reconnecting..."
            } else {
                status_el.textContent = "Reconnecting in " + left
            }
            break;
        default:
            status_el.textContent = state.status;
            break;
    }
}

function add_message(msg) {
    log('text', "[" + msg.author + "] " + msg.text)
}

function log(type, message) {
    let red = document.createElement('div');
    red.className = type;
    red.appendChild(document.createTextNode(message));
    mb.insertBefore(red, mb.childNodes[0]);
}

