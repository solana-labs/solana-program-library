"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.deleteMetadataEntryIx = exports.updateMetadataEntryIx = exports.createMetadataEntryIx = void 0;
const web3_js_1 = require("@solana/web3.js");
const borsh_1 = require("borsh");
const util_1 = require("./util");
function createMetadataEntryIx(programId, classKey, nameKey, targetProgramKey, targetProgramDataKey, targetProgramAuthorityKey, payerKey, systemProgramId, rentKey, nameServiceKey, name, value, hashedName) {
    const encodedName = Buffer.from(name);
    const encodedData = Buffer.from(value);
    let buffers = [
        Buffer.from(Int8Array.from([0])),
        new util_1.Numberu32(encodedName.length).toBuffer(),
        encodedName,
        new util_1.Numberu32(encodedData.length).toBuffer(),
        encodedData,
        new util_1.Numberu32(hashedName.length).toBuffer(),
        hashedName
    ];
    const ixData = Buffer.concat(buffers);
    const ix = new web3_js_1.TransactionInstruction({
        programId: programId,
        keys: [
            { pubkey: classKey, isSigner: false, isWritable: false },
            { pubkey: nameKey, isSigner: false, isWritable: true },
            { pubkey: targetProgramKey, isSigner: false, isWritable: false },
            { pubkey: targetProgramDataKey, isSigner: false, isWritable: false },
            { pubkey: targetProgramAuthorityKey, isSigner: true, isWritable: false },
            { pubkey: payerKey, isSigner: true, isWritable: true },
            { pubkey: systemProgramId, isSigner: false, isWritable: false },
            { pubkey: rentKey, isSigner: false, isWritable: false },
            { pubkey: nameServiceKey, isSigner: false, isWritable: false }
        ],
        data: Buffer.from(ixData)
    });
    return ix;
}
exports.createMetadataEntryIx = createMetadataEntryIx;
class UpdateMetadataEntryInstruction {
    constructor(value) {
        this.value = value;
        this.instruction = [1];
    }
}
const UpdateMetadataEntrySchema = new Map([[UpdateMetadataEntryInstruction, {
            kind: 'struct',
            fields: [
                ['instruction', [1]],
                ['value', 'string']
            ]
        }]]);
function updateMetadataEntryIx(programId, classKey, nameKey, targetProgramKey, targetProgramDataKey, targetProgramAuthorityKey, nameServiceKey, value) {
    const ixDataObject = new UpdateMetadataEntryInstruction(value);
    const ixData = borsh_1.serialize(UpdateMetadataEntrySchema, ixDataObject);
    const ix = new web3_js_1.TransactionInstruction({
        programId: programId,
        keys: [
            { pubkey: classKey, isSigner: false, isWritable: false },
            { pubkey: nameKey, isSigner: false, isWritable: true },
            { pubkey: targetProgramKey, isSigner: false, isWritable: false },
            { pubkey: targetProgramDataKey, isSigner: false, isWritable: false },
            { pubkey: targetProgramAuthorityKey, isSigner: true, isWritable: false },
            { pubkey: nameServiceKey, isSigner: false, isWritable: false }
        ],
        data: Buffer.from(ixData)
    });
    return ix;
}
exports.updateMetadataEntryIx = updateMetadataEntryIx;
function deleteMetadataEntryIx(programId, classKey, nameKey, targetProgramKey, targetProgramDataKey, targetProgramAuthorityKey, refundKey, nameServiceKey) {
    const ixData = Buffer.from(Int8Array.from([2]));
    const ix = new web3_js_1.TransactionInstruction({
        programId: programId,
        keys: [
            { pubkey: classKey, isSigner: false, isWritable: false },
            { pubkey: nameKey, isSigner: false, isWritable: true },
            { pubkey: targetProgramKey, isSigner: false, isWritable: false },
            { pubkey: targetProgramDataKey, isSigner: false, isWritable: false },
            { pubkey: targetProgramAuthorityKey, isSigner: true, isWritable: false },
            { pubkey: refundKey, isSigner: false, isWritable: false },
            { pubkey: nameServiceKey, isSigner: false, isWritable: false }
        ],
        data: ixData
    });
    return ix;
}
exports.deleteMetadataEntryIx = deleteMetadataEntryIx;
//# sourceMappingURL=instruction.js.map