"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.CreateMetadataEntryInstruction = void 0;
const borsh_struct_1 = require("../util/borsh-struct");
class CreateMetadataEntryInstruction extends borsh_struct_1.Struct {
    constructor(name, value, hashedName) {
        super();
        this.name = name;
        this.value = value;
        this.hashedName = hashedName;
        this.instruction = 0;
    }
}
exports.CreateMetadataEntryInstruction = CreateMetadataEntryInstruction;
borsh_struct_1.PROGRAM_METADATA_SCHEMA.set(CreateMetadataEntryInstruction, {
    kind: "struct",
    fields: [
        ["instruction", "u8"],
        ["name", "string"],
        ["value", "string"],
        ["hashedName", [32]],
    ],
});
//# sourceMappingURL=create-metadata-entry.js.map