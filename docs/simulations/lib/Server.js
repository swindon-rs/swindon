"use strict";

function Server(position) {
    this.position = position;
    this.queue = [];
    this.active = true;
    this.timerId = 0;
    this.gcPause = 0;
    this.latency = config.server.latency;
    this.requestCount = 0;
}

Server.prototype.draw = function (ctx) {
    const x = Math.round(this.position.x);
    const y = Math.round(this.position.y);

    ctx.fillStyle = "#090";
    if (this.gcPause > 0) {
        ctx.fillStyle = "#910";
    }
    if (this.color) {
        ctx.fillStyle = this.color;
    }
    ctx.beginPath();
    ctx.arc(x, y, config.circleSize, 0, Math.PI * 2);
    ctx.closePath();
    ctx.fill();

    for (let i = 0; i < Math.min(config.server.greenLength, this.queue.length); i++) {
        ctx.fillRect(x + 2 + config.circleSize, y - 4 * i, 10, 3)
    }
    if (this.queue.length > config.server.greenLength) {
        ctx.fillStyle = "#F00";
        ctx.fillRect(x + 2 + config.circleSize, y - 4 * config.server.greenLength, 10, 3)
    }
    ctx.fillText('' + this.queue.length, x + 15 + config.circleSize, y);
    if (this.gcPause) {
        ctx.fillText('GC Pause !!!', x + 15 + 2 * config.circleSize, y);
    }
}

Server.prototype.enqueue = function (request) {
    const self = this;
    request.delay = config.networkDelay;
    const processingLatency = self.latency(request) / config.speed;
    request.delay += processingLatency;
    this.queue.push(request);
    this.requestCount++;
    if (this.queue.length == 1) {
        setTimeout(() => process(self), processingLatency);
    }
}

Server.prototype.close = function () {
    clearTimeout(this.timerId);
}

Server.prototype.gc = function (howLongMs) {
    this.gcPause = this.gcPause + howLongMs;
}

Server.prototype.delayQueue = function (howLongMs) {
    for (const req of this.queue) {
        req.delay = req.delay + howLongMs;
    }
}

function process(server) {
    if (server.queue.length == 0 || !server.active) {
        return;
    }
    if (server.gcPause > 0) {
        const pause = server.gcPause / config.speed;
        server.timerId = setTimeout(() => {
            server.delayQueue(pause);
            server.gcPause = 0;
            process(server);
        }, pause);
        return;
    }

    const queueDepth = server.queue.length;
    const request = server.queue.splice(0, 1)[0];
    if (!request.src.active) {
        return process(server);
    }
    const latency = server.latency(request) / config.speed;
    server.delayQueue(latency);
    const source = request.src;
    request.src = request.dest;
    request.dest = source;

    request.response = true;
    request.sendTime = Date.now();
    requests.add(request);

    if (server.queue.length > 0) {
        server.timerId = setTimeout(() => {
            process(server);
        }, latency);
    }
}
