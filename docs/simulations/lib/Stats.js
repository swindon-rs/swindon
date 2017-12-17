'use strict';

function Stats(serverId) {
    this._serverId = serverId;
    this.load = 0;
    this._outstandings = 0;

    this._stamp = Date.now();        // last timestamp we sent a request
    this._stamp0 = this._stamp;      // last ts we sent a req or receive a resp.
    this._instantaneousDuration = 0; // instantaneous cumulative duration
    this._inactivityPeriodMs = 5000;

    this._median = new SlidingMedian(8);
    this._epoch = Date.now();
}

Stats.prototype.incr = function () {
    const now = Date.now();
    this.load += 1;

    //console.log((now - this._epoch) + " INCR (" + this._serverId + ") outstandings = " + this._outstandings + " duration = " + this._instantaneousDuration + " duration += " + (now - this._stamp0) * this._outstandings);
    this._instantaneousDuration += (now - this._stamp0) * this._outstandings;
    if (this._instantaneousDuration < 0) {
        debugger;
    }
    this._outstandings += 1;
    this._stamp = now;
    this._stamp0 = now;
}

Stats.prototype.decr = function (ts) {
    const now = Date.now();
    const rtt = now - ts;
    if (rtt < 0) {
        debugger;
    }
    this.load -= 1;

    const timeSinceLastUpdate = now - this._stamp0;
    //console.log((now - this._epoch) + " DECR (" + this._serverId + ") outstandings = " + this._outstandings + " duration = " + this._instantaneousDuration + " duration += " + timeSinceLastUpdate * this._outstandings);
    this._instantaneousDuration += timeSinceLastUpdate * this._outstandings;
    if (this._instantaneousDuration < 0) {
        debugger;
    }
    //console.log((now - this._epoch) + " DECR (" + this._serverId + ") outstandings = " + this._outstandings + " duration = " + this._instantaneousDuration + " duration -= " + rtt);
    this._instantaneousDuration -= rtt;
    this._instantaneousDuration = Math.max(0, this._instantaneousDuration);
    this._outstandings -= 1;
    this._outstandings = Math.max(0, this._outstandings); // when deleting servers
    this._stamp0 = now;
    this._median.insert(rtt);
}

Stats.prototype.median = function () {
    return this._median.estimate();
}

Stats.prototype.predictiveLoad0 = function () {
    const now = Date.now();
    const elapsed = now - this._stamp;
    const STARTUP_PENALTY = 100000 / 2;

    const prediction = this._median.estimate();
    let weight = prediction;

    if (prediction === 0) { // no data point yet
        if (this._outstandings === 0) {
            weight = 0; // first request
        } else {
            // subsequent requests while we don't have any history
            weight = STARTUP_PENALTY + this._outstandings;
        }
    }

    return weight * (this._outstandings + 1);
}

Stats.prototype.instantaneousDuration = function () {
    return this._instantaneousDuration + elapsed * this._outstandings;
}

Stats.prototype.predictiveLoad = function () {
    const now = Date.now();
    const elapsed = now - this._stamp;
    const STARTUP_PENALTY = 100000 / 2 - 1;

    let weight = 0;
    const prediction = this._median.estimate();

    if (prediction === 0) { // no data point yet
        if (this._outstandings === 0) {
            weight = 0; // first request
        } else {
            // subsequent requests while we don't have any history
            weight = STARTUP_PENALTY + this._outstandings;
        }
    } else if (this._outstandings === 0
        && elapsed > config.predictive.inactivityPeriodMs) {
        // if we did't see any data for a while, we decay the prediction by
        // inserting artificial low value into the median
        const lowerMedian = this._median.estimate() * 0.9;
        this._median.insert(lowerMedian);
        this._stamp = now;
        this._stamp0 = now;
        weight = this._median.estimate();
        console.log("Decaying median..." + lowerMedian);
    } else {
        const predicted = prediction * this._outstandings;
        const elapsed = now - this._stamp0;
        const instant = this._instantaneousDuration + elapsed * this._outstandings;
        if (instant < 0) {
            debugger;
        }

        if (this._outstandings != 0 && predicted < instant) { // NB: (0.0 < 0.0) == false
            // NB: _outstandings never equal 0 here
            weight = instant / this._outstandings;
        } else {
            // we are under the predictions
            weight = prediction;
        }
    }

    return weight * (this._outstandings + 1);
}

