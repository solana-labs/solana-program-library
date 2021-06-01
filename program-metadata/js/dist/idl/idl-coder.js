"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.IdlCoder = exports.CODER_MAP = exports.SerializationMethod = void 0;
const borsh_1 = require("./coders/borsh");
var SerializationMethod;
(function (SerializationMethod) {
    SerializationMethod[SerializationMethod["Bincode"] = 0] = "Bincode";
    SerializationMethod[SerializationMethod["Borsh"] = 1] = "Borsh";
    SerializationMethod[SerializationMethod["Anchor"] = 2] = "Anchor";
})(SerializationMethod = exports.SerializationMethod || (exports.SerializationMethod = {}));
exports.CODER_MAP = new Map([
    [SerializationMethod.Borsh, borsh_1.Borsh]
]);
class IdlCoder {
    constructor(idl, serializationMethod) {
        this.idl = idl;
        this.serializationMethod = serializationMethod;
        const coder = exports.CODER_MAP.get(serializationMethod);
        if (!coder) {
            throw new Error("Serialization method not supported");
        }
        this.coder = new coder(idl);
    }
    decodeInstruction(instruction) {
        return this.coder.decodeInstruction(instruction);
    }
    decodeAccount(account) {
    }
}
exports.IdlCoder = IdlCoder;
//# sourceMappingURL=idl-coder.js.map