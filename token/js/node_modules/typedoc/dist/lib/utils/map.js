"use strict";
var _a;
Object.defineProperty(exports, "__esModule", { value: true });
exports.StableKeyMap = exports.DefaultMap = void 0;
class DefaultMap extends Map {
    constructor(creator) {
        super();
        this.creator = creator;
    }
    get(key) {
        const saved = super.get(key);
        if (saved != null) {
            return saved;
        }
        const created = this.creator();
        this.set(key, created);
        return created;
    }
    getNoInsert(key) {
        return super.get(key);
    }
}
exports.DefaultMap = DefaultMap;
class StableKeyMap {
    constructor() {
        this[_a] = "StableKeyMap";
        this.impl = new Map();
    }
    get size() {
        return this.impl.size;
    }
    set(key, value) {
        this.impl.set(key.getStableKey(), [key, value]);
        return this;
    }
    get(key) {
        return this.impl.get(key.getStableKey())?.[1];
    }
    has(key) {
        return this.get(key) != null;
    }
    clear() {
        this.impl.clear();
    }
    delete(key) {
        return this.impl.delete(key.getStableKey());
    }
    forEach(callbackfn, thisArg) {
        for (const [k, v] of this.entries()) {
            callbackfn.apply(thisArg, [v, k, this]);
        }
    }
    entries() {
        return this.impl.values();
    }
    *keys() {
        for (const [k] of this.entries()) {
            yield k;
        }
    }
    *values() {
        for (const [, v] of this.entries()) {
            yield v;
        }
    }
    [(_a = Symbol.toStringTag, Symbol.iterator)]() {
        return this.entries();
    }
}
exports.StableKeyMap = StableKeyMap;
