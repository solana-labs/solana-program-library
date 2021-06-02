"use strict";
/**
 * TODO: export from anchor library or begin seperate module
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.IdlError = exports.SerializationMethod = void 0;
var SerializationMethod;
(function (SerializationMethod) {
    SerializationMethod["Bincode"] = "bincode";
    SerializationMethod["Borsh"] = "borsh";
    SerializationMethod["Anchor"] = "anchor";
})(SerializationMethod = exports.SerializationMethod || (exports.SerializationMethod = {}));
class IdlError extends Error {
}
exports.IdlError = IdlError;
//# sourceMappingURL=idl.js.map