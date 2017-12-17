"use strict";

function Request(src, dest, work, sendTime) {
    this.src = src;
    this.dest = dest;
    this.work = work;
    this.delay = 0;
    this.position = {x: src.position.x, y: src.position.y};
    this.sendTime = sendTime;
    this.srcId = 0;
    this.destId = 0;
}

Request.prototype.update = function (now) {
    const t = now - this.sendTime;
    if (t > config.networkDelay / config.speed) {
        requests.delete(this);
        if (this.response) {
            this.dest.load[this.destId] = (this.dest.load[this.destId] - 1) || 0;
            // console.log("DECR load[" + this.destId + "] = " + this.dest.load[this.destId]);
            // this.delay = networkLatency + processingTime
            const latency = this.delay + config.networkDelay
            if (this.dest.stats[this.destId]) { // if the server hasn't been deleted
                this.dest.stats[this.destId].decr(Math.max(0, this.originalSendTime));
            }
            histogram.add(latency);
            latencyPlot.add(now, latency);
        } else {
            this.dest.enqueue(this);
        }
    } else {
        const dx = this.dest.position.x - this.src.position.x;
        const dy = this.dest.position.y - this.src.position.y;
        const k = config.speed * t / config.networkDelay;
        this.position.x = this.src.position.x + k * dx;
        this.position.y = this.src.position.y + k * dy;
    }
}

Request.prototype.draw = function (ctx) {
    var position = this.position;
    ctx.fillStyle = "#88F";
    ctx.fillRect(position.x, position.y, config.particleSize, config.particleSize);
}
