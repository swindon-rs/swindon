//@ts-check
'use strict';

function LatencyPlot(opts) {
    this.position = opts.position;
    this.dimension = opts.dimension;
    this.windowMs = opts.windowMs;
    this.buffer = [];
    this.slidingMedian = new SlidingMedian(16);

    this.ewma = 0;
    this.prevTime = 0;
}

LatencyPlot.prototype.add = function (t, x) {
    this.slidingMedian.insert(x);
    const m = this.slidingMedian.estimate();

    const now = Date.now();
    const dt = now - this.prevTime;
    this.prevTime = now;
    const w = Math.exp(-dt/75)
    this.ewma = w * this.ewma + (1.0 - w) * m;

    this.buffer.push({
        t: t,
        median: this.ewma,
        x: x
    });
}

LatencyPlot.prototype.update = function (now) {
    const cutoff = now - this.windowMs;
    let i = 0;
    for (i = 0; i<this.buffer.length; i++) {
        if (this.buffer[i].t > cutoff) {
            break;
        }
    }
    this.buffer.splice(0, i);
}

LatencyPlot.prototype.draw = function (ctx) {
    const self = this;
    ctx.fillStyle = "#000";

    // Y Axes
    ctx.fillRect(this.position.x, this.position.y, this.dimension.x, 1);
    ctx.beginPath();
    ctx.moveTo(this.position.x, this.position.y - this.dimension.y);
    ctx.lineTo(this.position.x - 5, this.position.y - this.dimension.y);
    ctx.lineTo(this.position.x, this.position.y - this.dimension.y - 10);
    ctx.lineTo(this.position.x + 5, this.position.y - this.dimension.y);
    ctx.lineTo(this.position.x, this.position.y - this.dimension.y);
    ctx.closePath();
    ctx.fill();

    // X Axis
    ctx.fillRect(this.position.x, this.position.y - this.dimension.y, 1, this.dimension.y);
    ctx.beginPath();
    ctx.moveTo(this.position.x + this.dimension.x, this.position.y);
    ctx.lineTo(this.position.x + this.dimension.x, this.position.y - 5);
    ctx.lineTo(this.position.x + this.dimension.x + 10, this.position.y);
    ctx.lineTo(this.position.x + this.dimension.x, this.position.y + 5);
    ctx.lineTo(this.position.x + this.dimension.x, this.position.y);
    ctx.closePath();
    ctx.fill();

    ctx.fillText("Latency Plot", this.position.x + this.dimension.x / 3, this.position.y + config.fontSize);

    const n = this.buffer.length;
    if (n > 0) {
        const first = this.buffer[0];
        const last = this.buffer[n - 1];
        const timeRange = last.t - first.t;
        let maxValue = 0;
        let minValue = Number.MAX_VALUE;
        for (let i = 0; i < this.buffer.length; i++) {
            maxValue = Math.max(this.buffer[i].x, maxValue);
            minValue = Math.min(this.buffer[i].x, minValue);
        }
        let valueRange = maxValue - minValue;
        if (!valueRange) {
            valueRange = 1;
        }

        let dataBuffer = [];

        function rescale(value) {
            const yy = (value - minValue) / valueRange;
            const yyy = (Math.exp(yy) - 1) / (Math.E - 1);

            return yyy * self.dimension.y;
        }

        for (let i = 0; i < this.buffer.length; i++) {
            const datapoint = this.buffer[i];
            const tx = (datapoint.t - first.t) / timeRange;
            const xOffset = this.dimension.x * tx;
            const x = Math.round(this.position.x + xOffset);

            let yOffset = rescale(datapoint.x);
            dataBuffer.push(yOffset);

            if (yOffset == 0) {
                yOffset = 5; // offset the point at 0 to avoid x axis overlap
            }
            const y = Math.round(this.position.y - yOffset);
            ctx.fillStyle = "#000";
            ctx.fillRect(x - 2, y, 6, 2);
            ctx.fillRect(x, y - 2, 2, 6);
        }

        if (config.showMedianGraph && this.buffer.length > 1) {
            ctx.beginPath();
            const datapoint0 = this.buffer[0];
            let yMedianOffset0 = rescale(datapoint0.median);
            const yMedian0 = Math.round(this.position.y - yMedianOffset0);
            const tx0 = (datapoint0.t - first.t) / timeRange;
            const xOffset0 = this.dimension.x * tx0;
            const x0 = Math.round(this.position.x + xOffset0);

            ctx.moveTo(x0, yMedian0);
            for (let i = 0; i < this.buffer.length; i++) {
                const datapoint = this.buffer[i];
                const tx = (datapoint.t - first.t) / timeRange;
                const xOffset = this.dimension.x * tx;
                const x = Math.round(this.position.x + xOffset);

                let yMedianOffset = rescale(datapoint.median);
                const yMedian = Math.round(this.position.y - yMedianOffset);
                //ctx.fillRect(x - 1, yMedian, 3, 3);

                ctx.lineTo(x, yMedian);
            }
            ctx.lineWidth=3;
            ctx.strokeStyle = "#E00";
            ctx.stroke();
        }

        dataBuffer.sort((a, b) => (a - b));
        const k = Math.floor(dataBuffer.length * 0.9);
        const p90 = dataBuffer[k];

        if (config.showPercentile) {
            ctx.fillStyle = "#0D0";
            const p90y = Math.round(this.position.y - p90);
            ctx.fillText("10 % (" + (dataBuffer.length - k) + " points)", this.position.x + this.dimension.x / 2, p90y - config.fontSize + 15);
            ctx.fillText("90 % (" + k + " points)", this.position.x + this.dimension.x / 2, p90y + config.fontSize);
            ctx.fillRect(this.position.x, p90y, this.position.x + this.dimension.x, 1);
        }
    }
}
