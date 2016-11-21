+function() {

    var ws = new WebSocket("ws://" + location.host + "/")
    var mb = document.getElementById('mb');
    var input = document.getElementById('input');
    var my_user_id = null;
    ws.onopen = function() {
        log('debug', "Connected")
    }

    ws.onclose = function() {
        input.style.visibility = 'hidden'
        log('warning', "Disconnected")
    }

    ws.onerror = function(e) {
        input.style.visibility = 'hidden'
        log('warning', 'ERROR: ' + e)
    }
    ws.onmessage = function(ev) {
        var data = JSON.parse(ev.data)
        if(data[0] == 'hello') {
            my_user_id = data[2]['user_id']
            log('info', "Your name is " + data[2]['username'])
            input.style.visibility = 'visible'
            input.focus()
        } else {
            console.error("Unknown message", data)
        }
    }
    input.onkeydown = function(ev) {
        if(ev.which == 13) {
            ws.send(JSON.stringify([
                "message",     // method
                {'request_id': 1},            // metadata
                [input.value], // args
                {},            // kwargs
            ]))
            input.value = ''
        }
    }


    function log(type, message) {
        let red = document.createElement('div');
        red.className = type;
        red.appendChild(document.createTextNode(message));
        mb.insertBefore(red, mb.childNodes[0]);
    }

}()
