"use strict";

function toHexaColor(intensity) {
    let strIntensity = Math.floor(255 * intensity).toString(16);
    if (strIntensity.length < 2) {
        strIntensity = "0" + strIntensity;
    }
    return '#ff' + strIntensity + strIntensity;
}

function HeatMap(opts) {
    this.windowHistogram = opts.windowHistogram
    this.position = opts.position;
    this.dimension = opts.dimension;
}

HeatMap.prototype.draw = function (ctx) {
    const numberOfBuckets = this.windowHistogram.count
    const bucketWidth = Math.floor(this.dimension.x / numberOfBuckets);
    const maxValue = this.windowHistogram.maxValue() + 1;

    const height = 15;
    let buffers = [];
    let maxCount = 0;
    for (let i = 0; i < numberOfBuckets; i++) {
        const histo = this.windowHistogram.histograms[(i + this.windowHistogram.lastIndex + 1) % this.windowHistogram.histograms.length];
        const data0 = _.take(histo.buffer, maxValue);
        const data = scaleX(data0, this.dimension.y, height);
        buffers.push(data);
        maxCount = Math.max(maxCount, _.max(data));
    }

    let offset = 0;
    for (let i = 0; i < buffers.length; i++) {
        const data = buffers[i];
        for (let j = 0; j < maxValue; j++) {
            const count = data[j];
            if (count > 0) {
                const x = (maxCount - count) / maxCount;
                const intensity = config.heatMap.colorScaling(x);
                const fillStyle = toHexaColor(intensity);
                ctx.fillStyle = fillStyle;
                ctx.fillRect(this.position.x + offset, this.position.y - (j + 1) * height, bucketWidth, height);
            }
        }
        offset = offset + bucketWidth;
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

}
