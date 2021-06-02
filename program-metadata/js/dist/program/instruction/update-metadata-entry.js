"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.UpdateMetadataEntryInstruction = void 0;
const borsh_struct_1 = require("../util/borsh-struct");
class UpdateMetadataEntryInstruction extends borsh_struct_1.Struct {
    constructor(value) {
        super();
        this.value = value;
        this.instruction = 1;
    }
}
exports.UpdateMetadataEntryInstruction = UpdateMetadataEntryInstruction;
borsh_struct_1.PROGRAM_METADATA_SCHEMA.set(UpdateMetadataEntryInstruction, {
    kind: "struct",
    fields: [
        ["instruction", "u8"],
        ["value", "string"],
    ],
});
//# sourceMappingURL=update-metadata-entry.js.map