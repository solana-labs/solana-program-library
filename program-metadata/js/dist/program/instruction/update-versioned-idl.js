"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.UpdateVersionedIdlInstruction = void 0;
const borsh_struct_1 = require("../util/borsh-struct");
class UpdateVersionedIdlInstruction extends borsh_struct_1.Struct {
    constructor(idlUrl, idlHash, sourceUrl, serialization, customLayoutUrl) {
        super();
        this.idlUrl = idlUrl;
        this.idlHash = idlHash;
        this.sourceUrl = sourceUrl;
        this.customLayoutUrl = customLayoutUrl;
        this.instruction = 4;
        this.serialization = [serialization];
    }
}
exports.UpdateVersionedIdlInstruction = UpdateVersionedIdlInstruction;
borsh_struct_1.PROGRAM_METADATA_SCHEMA.set(UpdateVersionedIdlInstruction, {
    kind: "struct",
    fields: [
        ["instruction", 'u8'],
        ["idlUrl", "string"],
        ["idlHash", [32]],
        ["sourceUrl", "string"],
        ["serialization", [1]],
        [
            "customLayoutUrl",
            {
                kind: "option",
                type: "string",
            },
        ],
    ],
});
//# sourceMappingURL=update-versioned-idl.js.map