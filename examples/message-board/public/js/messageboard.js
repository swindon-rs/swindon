+function() {

    var ws = new WebSocket("ws://" + location.host + "/")
    ws.onopen = function() {
        console.log("Connected")
    }

}()
