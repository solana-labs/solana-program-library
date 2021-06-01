"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.PROGRAM_METADATA_SCHEMA = exports.Struct = void 0;
const buffer_1 = require("buffer");
const borsh_1 = require("borsh");
// Class wrapping a plain object
class Struct {
    encode() {
        return buffer_1.Buffer.from(borsh_1.serialize(exports.PROGRAM_METADATA_SCHEMA, this));
    }
    static decode(data) {
        return borsh_1.deserialize(exports.PROGRAM_METADATA_SCHEMA, this, data);
    }
}
exports.Struct = Struct;
exports.PROGRAM_METADATA_SCHEMA = new Map();
//# sourceMappingURL=borsh-struct.js.map