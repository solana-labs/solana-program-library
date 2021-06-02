"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.updateVersionedIdlIx = exports.createVersionedIdlIx = exports.deleteMetadataEntryIx = exports.updateMetadataEntryIx = exports.createMetadataEntryIx = void 0;
const web3_js_1 = require("@solana/web3.js");
const update_metadata_entry_1 = require("./instruction/update-metadata-entry");
const create_versioned_idl_1 = require("./instruction/create-versioned-idl");
const update_versioned_idl_1 = require("./instruction/update-versioned-idl");
const create_metadata_entry_1 = require("./instruction/create-metadata-entry");
const delete_metadata_entry_1 = require("./instruction/delete-metadata-entry");
function createMetadataEntryIx(programId, classKey, nameKey, targetProgramKey, targetProgramDataKey, targetProgramAuthorityKey, payerKey, systemProgramId, rentKey, nameServiceKey, name, value, hashedName) {
    const ixDataObject = new create_metadata_entry_1.CreateMetadataEntryInstruction(name, value, hashedName);
    const ixData = ixDataObject.encode();
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
            { pubkey: nameServiceKey, isSigner: false, isWritable: false },
        ],
        data: ixData,
    });
    return ix;
}
exports.createMetadataEntryIx = createMetadataEntryIx;
function updateMetadataEntryIx(programId, classKey, nameKey, targetProgramKey, targetProgramDataKey, targetProgramAuthorityKey, nameServiceKey, value) {
    const ixDataObject = new update_metadata_entry_1.UpdateMetadataEntryInstruction(value);
    const ixData = ixDataObject.encode();
    const ix = new web3_js_1.TransactionInstruction({
        programId: programId,
        keys: [
            { pubkey: classKey, isSigner: false, isWritable: false },
            { pubkey: nameKey, isSigner: false, isWritable: true },
            { pubkey: targetProgramKey, isSigner: false, isWritable: false },
            { pubkey: targetProgramDataKey, isSigner: false, isWritable: false },
            { pubkey: targetProgramAuthorityKey, isSigner: true, isWritable: false },
            { pubkey: nameServiceKey, isSigner: false, isWritable: false },
        ],
        data: ixData,
    });
    return ix;
}
exports.updateMetadataEntryIx = updateMetadataEntryIx;
function deleteMetadataEntryIx(programId, classKey, nameKey, targetProgramKey, targetProgramDataKey, targetProgramAuthorityKey, refundKey, nameServiceKey) {
    const ixDataObject = new delete_metadata_entry_1.DeleteMetadataEntry();
    const ixData = ixDataObject.encode();
    const ix = new web3_js_1.TransactionInstruction({
        programId: programId,
        keys: [
            { pubkey: classKey, isSigner: false, isWritable: false },
            { pubkey: nameKey, isSigner: false, isWritable: true },
            { pubkey: targetProgramKey, isSigner: false, isWritable: false },
            { pubkey: targetProgramDataKey, isSigner: false, isWritable: false },
            { pubkey: targetProgramAuthorityKey, isSigner: true, isWritable: false },
            { pubkey: refundKey, isSigner: false, isWritable: false },
            { pubkey: nameServiceKey, isSigner: false, isWritable: false },
        ],
        data: ixData,
    });
    return ix;
}
exports.deleteMetadataEntryIx = deleteMetadataEntryIx;
function createVersionedIdlIx(programId, classKey, nameKey, targetProgramKey, targetProgramDataKey, targetProgramAuthorityKey, payerKey, systemProgramId, rentKey, nameServiceKey, effectiveSlot, idlUrl, idlHash, sourceUrl, hashedName) {
    const ixDataObject = new create_versioned_idl_1.CreateVersionedIdlInstruction(effectiveSlot, idlUrl, idlHash, sourceUrl, hashedName);
    const ixData = ixDataObject.encode();
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
            { pubkey: nameServiceKey, isSigner: false, isWritable: false },
        ],
        data: ixData,
    });
    return ix;
}
exports.createVersionedIdlIx = createVersionedIdlIx;
function updateVersionedIdlIx(programId, classKey, nameKey, targetProgramKey, targetProgramDataKey, targetProgramAuthorityKey, nameServiceKey, idlUrl, idlHash, sourceUrl) {
    const ixDataObject = new update_versioned_idl_1.UpdateVersionedIdlInstruction(idlUrl, idlHash, sourceUrl);
    const ixData = ixDataObject.encode();
    const ix = new web3_js_1.TransactionInstruction({
        programId: programId,
        keys: [
            { pubkey: classKey, isSigner: false, isWritable: false },
            { pubkey: nameKey, isSigner: false, isWritable: true },
            { pubkey: targetProgramKey, isSigner: false, isWritable: false },
            { pubkey: targetProgramDataKey, isSigner: false, isWritable: false },
            { pubkey: targetProgramAuthorityKey, isSigner: true, isWritable: false },
            { pubkey: nameServiceKey, isSigner: false, isWritable: false },
        ],
        data: ixData,
    });
    return ix;
}
exports.updateVersionedIdlIx = updateVersionedIdlIx;
//# sourceMappingURL=instruction.js.map