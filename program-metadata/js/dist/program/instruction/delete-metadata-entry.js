"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.DeleteMetadataEntry = void 0;
const borsh_struct_1 = require("../util/borsh-struct");
class DeleteMetadataEntry extends borsh_struct_1.Struct {
    constructor() {
        super(...arguments);
        this.instruction = 2;
    }
}
exports.DeleteMetadataEntry = DeleteMetadataEntry;
borsh_struct_1.PROGRAM_METADATA_SCHEMA.set(DeleteMetadataEntry, {
    kind: "struct",
    fields: [["instruction", "u8"]],
});
//# sourceMappingURL=delete-metadata-entry.js.map