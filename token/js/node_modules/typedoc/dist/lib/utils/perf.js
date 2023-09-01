"use strict";
/* eslint-disable no-console */
Object.defineProperty(exports, "__esModule", { value: true });
exports.measure = exports.Bench = exports.bench = void 0;
const perf_hooks_1 = require("perf_hooks");
const benchmarks = [];
function bench(fn, name = fn.name) {
    const timer = {
        name,
        calls: 0,
        time: 0,
    };
    benchmarks.push(timer);
    return function bench(...args) {
        timer.calls++;
        const start = perf_hooks_1.performance.now();
        let result;
        try {
            result = fn.apply(this, args);
        }
        finally {
            timer.time += perf_hooks_1.performance.now() - start;
        }
        return result;
    };
}
exports.bench = bench;
function Bench() {
    return function (target, key, descriptor) {
        const rawMethod = descriptor.value;
        const name = `${target.name ?? target.constructor.name}.${String(key)}`;
        descriptor.value = bench(rawMethod, name);
    };
}
exports.Bench = Bench;
const anon = { name: "measure()", calls: 0, time: 0 };
function measure(cb) {
    if (anon.calls === 0) {
        benchmarks.unshift(anon);
    }
    anon.calls++;
    const start = perf_hooks_1.performance.now();
    let result;
    try {
        result = cb();
    }
    finally {
        anon.time += perf_hooks_1.performance.now() - start;
    }
    return result;
}
exports.measure = measure;
process.on("beforeExit", () => {
    if (!benchmarks.length)
        return;
    const width = benchmarks.reduce((a, b) => Math.max(a, b.name.length), 11);
    console.log("=".repeat(width + 20));
    console.log(`${"Benchmarked".padEnd(width)} | Calls | Time`);
    console.log("=".repeat(width + 20));
    for (const { name, calls, time } of benchmarks) {
        console.log(`${name.padEnd(width)} | ${calls
            .toString()
            .padEnd(5)} | ${time.toFixed(2)}ms`);
    }
    console.log("=".repeat(width + 20));
});
