"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.IdlCoder = exports.CODER_MAP = void 0;
const borsh_1 = require("./coders/borsh");
const idl_1 = require("./idl");
const DEFAULT_SERIALIZATION_METHOD = idl_1.SerializationMethod.Anchor;
exports.CODER_MAP = new Map([
    [idl_1.SerializationMethod.Borsh, borsh_1.Borsh],
]);
class IdlCoder {
    constructor(idl) {
        this.idl = idl;
        const serializationMethod = idl.serializationMethod || DEFAULT_SERIALIZATION_METHOD;
        const coder = exports.CODER_MAP.get(serializationMethod);
        if (!coder) {
            throw new Error("Serialization method not supported");
        }
        this.coder = new coder(idl);
    }
    decodeInstruction(instruction) {
        return this.coder.decodeInstruction(instruction);
    }
}
exports.IdlCoder = IdlCoder;
//# sourceMappingURL=idl-coder.js.map