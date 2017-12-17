"use strict";

const config = {
    screenCuts: 4,
    fontSize: 30,
    circleSize: 20,
    particleSize: 14,
    bottomOffset: 200,
    drawHeader: true,
    drawAnimation: true,

    loadbalancing: "random",
    networkDelay: 750,

    alignGraphs: true,
    showHistogram: false,
    showHeatMap: false,
    showLatencyPlot: true,
    histogram: {
        yScaling: x => x
    },
    heatMap: {
        colorScaling: x => x
    },
    windowCount: 30,
    windowMs: 30000,

    source: {
        number: 1,
        globalRps: 5,
        work: function () {
            return 50;
        }
    },

    server: {
        number: 1,
        greenLength: 10,
        latency: function (req) {
            return Math.abs(Math.round(req.work + (100 * normalRandom())));
        }
    },

    aperture: {
        minConnections: 3,
        maxConnections: 20,
        minRatio: 1,
        maxRatio: 2,
        minRefreshPeriod: 1000,
        refreshPeriod: 3000
    },

    predictive: {
        inactivityPeriodMs: 1000
    },

    speed: 1,
    clockSpeed: 1,
    emitPaused: false
};
