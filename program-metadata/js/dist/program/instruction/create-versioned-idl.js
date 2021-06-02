"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.CreateVersionedIdlInstruction = void 0;
const borsh_struct_1 = require("../util/borsh-struct");
class CreateVersionedIdlInstruction extends borsh_struct_1.Struct {
    constructor(effectiveSlot, idlUrl, idlHash, sourceUrl, hashedName) {
        super();
        this.effectiveSlot = effectiveSlot;
        this.idlUrl = idlUrl;
        this.idlHash = idlHash;
        this.sourceUrl = sourceUrl;
        this.hashedName = hashedName;
        this.instruction = 3;
    }
}
exports.CreateVersionedIdlInstruction = CreateVersionedIdlInstruction;
borsh_struct_1.PROGRAM_METADATA_SCHEMA.set(CreateVersionedIdlInstruction, {
    kind: "struct",
    fields: [
        ["instruction", "u8"],
        ["effectiveSlot", "u64"],
        ["idlUrl", "string"],
        ["idlHash", [32]],
        ["sourceUrl", "string"],
        ["hashedName", [32]],
    ],
});
//# sourceMappingURL=create-versioned-idl.js.map