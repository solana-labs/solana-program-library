"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.Assignable = exports.Numberu32 = void 0;
const bn_js_1 = __importDefault(require("bn.js"));
const assert = require('assert');
class Numberu32 extends bn_js_1.default {
    constructor(n) {
        super(n);
    }
    /**
   * Convert to Buffer representation
   */
    toBuffer() {
        const a = super.toArray().reverse();
        const b = Buffer.from(a);
        if (b.length === 4) {
            return b;
        }
        assert(b.length < 4, "Numberu32 too large");
        const zeroPad = Buffer.alloc(4);
        b.copy(zeroPad);
        return zeroPad;
    }
    /**
     * Construct a Numberu64 from Buffer representation
     */
    static fromBuffer(buffer) {
        assert(buffer.length === 4, `Invalid buffer length: ${buffer.length}`);
        return new bn_js_1.default([...buffer]
            .reverse()
            .map((i) => `00${i.toString(16)}`.slice(-2))
            .join(""), 16);
    }
}
exports.Numberu32 = Numberu32;
class Assignable {
    constructor(properties) {
        Object.keys(properties).map((key) => {
            this[key] = properties[key];
        });
    }
}
exports.Assignable = Assignable;
//# sourceMappingURL=util.js.map