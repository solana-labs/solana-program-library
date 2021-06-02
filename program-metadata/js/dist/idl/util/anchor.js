"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.sighash = void 0;
// Not technically sighash, since we don't include the arguments, as Rust
// doesn't allow function overloading.
function sighash(nameSpace, ixName) {
    let name = snakeCase(ixName);
    let preimage = `${nameSpace}:${name}`;
    return Buffer.from(sha256.digest(preimage)).slice(0, 8);
}
exports.sighash = sighash;
//# sourceMappingURL=anchor.js.map