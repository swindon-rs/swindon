"use strict";

function Histogram(opts) {
    this.bucketSize = opts.bucketSize;
    const size = Math.floor(opts.maxDuration / opts.bucketSize);
    this.max = size * opts.bucketSize;
    this.buffer = new Array(size);
    for (let i = 0; i < size; i++) {
        this.buffer[i] = 0;
    }
    this.maxCount = 0;
    this.maxValue = 0;
}

Histogram.prototype.add = function (x) {
    if (x > this.max) {
        // console.log('Histogram overflow, ' + x + ' > max(' + this.max + ')');
        return;
    }
    const i = Math.floor(x / this.bucketSize);
    this.buffer[i] = this.buffer[i] + 1;
    this.maxCount = Math.max(this.maxCount, this.buffer[i]);
    this.maxValue = Math.max(this.maxValue, i);
}

Histogram.prototype.clear = function () {
    for (let i = 0; i < this.buffer.length; i++) {
        this.buffer[i] = 0;
    }
    this.maxCount = 0;
    this.maxValue = 0;
}

Histogram.prototype.merge = function (histogram) {
    const result = new Histogram(this.max, this.bucketSize)
    for (let i=0; i<result.buffer.length; i++) {
        result.buffer[i] = this.buffer[i] + histogram.buffer[i];
    }
    return result;
}


function WindowHistogram(opts) {
    // histogramFactory, count, windowMs, position, dimension, clock
    this.histograms = [];
    for (let i = 0; i < opts.count; i++) {
        this.histograms.push(opts.factory());
    }
    this.windowMs = opts.windowMs;
    this.clock = opts.clock || Date.now;
    this.count = opts.count;
    this.lastIndex = Math.floor(this.clock() / this.windowMs);
    this.position = opts.position;
    this.dimension = opts.dimension;
    this.p99 = 0;
}

WindowHistogram.prototype.add = function (x) {
    if (isNaN(x)) {
        debugger;
    }
    const t = this.clock();
    const index = Math.floor(t / this.windowMs)
    const n = this.histograms.length;
    if (index !== this.lastIndex) {
        for (let j=this.lastIndex + 1; j<=index; j++) {
            const h = this.histograms[j % n];
            h.clear();
        }
        this.lastIndex = index;
    }
    const i = (index % n) >> 0;
    this.histograms[i].add(x);
}

WindowHistogram.prototype.maxIndividualCount = function () {
    let count = 0;
    for (let i = 0; i < this.histograms.length; i++) {
        count = Math.max(count, this.histograms[i].maxCount);
    }
    return count;
}

WindowHistogram.prototype.maxCount = function () {
    let count = 0;
    for (let i = 0; i < this.histograms[0].buffer.length; i++) {
        count = Math.max(count, this.buffer(i) || 0);
    }
    return count || 0;
}

WindowHistogram.prototype.maxValue = function () {
    let value = 0;
    for (let i = 0; i < this.histograms.length; i++) {
        value = Math.max(value, this.histograms[i].maxValue);
    }
    return value;
}

WindowHistogram.prototype.clear = function () {
    for (let i = 0; i < this.histograms.length; i++) {
        this.histograms[i].clear();
    }
}

WindowHistogram.prototype.buffer = function (index) {
    let count = 0;
    for (let i = 0; i < this.histograms.length; i++) {
        const histo = this.histograms[i];
        count = count + histo.buffer[index];
    }

    return count;
}

WindowHistogram.prototype.data = function () {
    const n = this.maxValue() + 1;
    let result = new Array(n);
    for (let i = 0; i < result.length; i++) {
        result[i] = this.buffer(i);
    }

    return result;
}

function scaleX(data, width, minWidth) {
    const n = Math.floor(width / minWidth);
    const scaledData = new Array(n);
    const targetCredit = data.length / n;

    let j=0, i = 0;
    let credit = 0;
    let sum = 0;
    while (i < data.length) {
        credit++;
        if (credit >= targetCredit) {
            while (credit >= targetCredit) {
                credit = credit - targetCredit;

                if (targetCredit < 1 - credit) {
                    scaledData[j] = sum + targetCredit * data[i];
                } else {
                    scaledData[j] = sum + (1 - credit) * data[i];
                    sum = credit * data[i];
                }
                j++;
            }
        } else {
            sum = sum + data[i];
        }
        i++;
        if (i === data.length) {
            scaledData[j] = sum;
            if (scaledData[j] < 0) {
                debugger;
            }
        }
    }

    return scaledData;
}

function scaleY(data, height) {
    const maxCount = _.max(data);
    const hFactor = height / config.histogram.yScaling(maxCount);
    return _.map(data, x => { return config.histogram.yScaling(x) * hFactor; });
}

WindowHistogram.prototype.scale = function (data, dimension) {
    const scaledXData = scaleX(data, dimension.x, 15);
    const scaledXYData = scaleY(scaledXData, dimension.y);
    return _.map(scaledXYData, Math.round);
}

function percentiles(data, q) {
    const target = _.sum(data) * q;
    let sum=0;
    for (let i=0; i<data.length; i++) {
        sum = sum + data[i];
        if (sum > target) {
            const delta = (sum - target) / data[i]
            return i + delta;
        }
    }
    return data.length - 1;
}

WindowHistogram.prototype.draw = function (ctx) {
    ctx.fillStyle = "#EEEEEE";
    const delta = 25;
    for (let i=1; i<this.dimension.y/delta; i++) {
        ctx.fillRect(this.position.x, this.position.y - delta*i, this.dimension.x, 1);
    }

    const rawData = this.data();
    if (rawData.length > 0) {
        const data = this.scale(rawData, this.dimension);
        const scalingFactor = rawData.length / data.length;
        const barWidth = Math.max(1, Math.floor(this.dimension.x / data.length));

        ctx.fillStyle = "#900";
        for (let i = 0; i < data.length; i++) {
            const count = data[i];
            ctx.fillRect(this.position.x + 5 + barWidth * i, this.position.y - count, barWidth, count);
        }

        const p99 = percentiles(data, 0.99);
        this.p99 = (p99 * scalingFactor * this.histograms[0].bucketSize).toFixed(1);

        const p99Label = "p99=" + (p99 * scalingFactor * this.histograms[0].bucketSize).toFixed(1);
        const p99Position = Math.round(p99 * barWidth);
        ctx.fillStyle = "#0F0";
        ctx.fillText(p99Label, this.position.x + p99Position, this.position.y + config.fontSize);
        ctx.fillRect(this.position.x + p99Position, this.position.y - this.dimension.y, 1, this.dimension.y);
    }

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

    ctx.fillText("Latency Histogram", this.position.x + this.dimension.x / 3, this.position.y + config.fontSize);
}
