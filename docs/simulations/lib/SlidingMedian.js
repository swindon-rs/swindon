'use strict';

/**
 * SlidingMedian
 *
 * Compute a streaming median
 *
 * @param {Number} size number of elements to keep in the buffer
 * @returns {SlidingMedian}
 */
function SlidingMedian(size) {
    this._size = size || 32;
    this._buffer = [];
}

/**
 * Insert a value in the median estimator.
 *
 * @param {Number} x the value to insert.
 * @returns {null}
 */
SlidingMedian.prototype.insert = function insert(x) {
    if (this._buffer.length == 0) {
        this._buffer.push(x);
        return;
    }

    const median = this.estimate();
    this._buffer.push(x);
    this._buffer.sort((a, b) => a - b);

    if (this._buffer.length > this._size) {
        if (x <= median) {
            this._buffer.splice(this._buffer.length - 1, 1);
        } else {
            this._buffer.splice(0, 1);
        }
    }
};

/**
 * Estimate the current median value.
 *
 * @returns {Number} returns the current estimate or 0 if no values have been
 * inserted.
 */
SlidingMedian.prototype.estimate = function estimate() {
    if (this._buffer.length == 0) {
        return 0;
    }
    const midpoint = Math.floor(this._buffer.length / 2);
    return this._buffer[midpoint];
};
