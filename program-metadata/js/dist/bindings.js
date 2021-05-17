"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ProgramMetadata = exports.PROGRAM_METADATA_ID = exports.METADATA_PREFIX = exports.NS_PROGRAM_ID = exports.NS_HASH_PREFIX = void 0;
const web3_js_1 = require("@solana/web3.js");
const crypto_1 = require("crypto");
const instruction_1 = require("./instruction");
exports.NS_HASH_PREFIX = "SPL Name Service";
exports.NS_PROGRAM_ID = new web3_js_1.PublicKey('2eD37nsnRfY7QdymU6GXrkZ7rUhpL6Y29e8K8dhisN7G');
exports.METADATA_PREFIX = "program_metadata";
exports.PROGRAM_METADATA_ID = new web3_js_1.PublicKey('6cQ31NiNjrTTvjFbXiDUxo2ao29jQrGpV2JkN1Ztm2Gy');
class ProgramMetadata {
    constructor(connection, config) {
        this.connection = connection;
        this.programMetadataKey = exports.PROGRAM_METADATA_ID;
        this.nameServiceKey = exports.NS_PROGRAM_ID;
        if (config === null || config === void 0 ? void 0 : config.programMetadataKey) {
            this.programMetadataKey = config.programMetadataKey;
        }
        if (config === null || config === void 0 ? void 0 : config.nameServiceKey) {
            this.nameServiceKey = config.nameServiceKey;
        }
    }
    async createMetadataEntry(targetProgramId, targetProgramAuthorityKey, payerKey, name, value) {
        const hashedName = this.getHashedName(name);
        const classKey = await this.getClassKey(targetProgramId);
        const nameKey = await this.getNameKey(hashedName, classKey);
        const targetProgramAcct = await this.connection.getAccountInfo(targetProgramId);
        if (!targetProgramAcct) {
            throw new Error('Program not found');
        }
        const targetProgramDataKey = new web3_js_1.PublicKey(targetProgramAcct.data.slice(3));
        const ix = instruction_1.createMetadataEntryIx(this.programMetadataKey, classKey, nameKey, targetProgramId, targetProgramDataKey, targetProgramAuthorityKey, payerKey, web3_js_1.SystemProgram.programId, web3_js_1.SYSVAR_RENT_PUBKEY, this.nameServiceKey, name, value, hashedName);
        return ix;
    }
    async updateMetadataEntry(targetProgramId, targetProgramAuthorityKey, name, value) {
        const hashedName = this.getHashedName(name);
        const classKey = await this.getClassKey(targetProgramId);
        const nameKey = await this.getNameKey(hashedName, classKey);
        const targetProgramAcct = await this.connection.getAccountInfo(targetProgramId);
        if (!targetProgramAcct) {
            throw new Error('Program not found');
        }
        const targetProgramDataKey = new web3_js_1.PublicKey(targetProgramAcct.data.slice(3));
        const ix = instruction_1.updateMetadataEntryIx(this.programMetadataKey, classKey, nameKey, targetProgramId, targetProgramDataKey, targetProgramAuthorityKey, this.nameServiceKey, value);
        return ix;
    }
    async deleteMetadataEntry(targetProgramId, targetProgramAuthorityKey, refundKey, name) {
        const hashedName = this.getHashedName(name);
        const classKey = await this.getClassKey(targetProgramId);
        const nameKey = await this.getNameKey(hashedName, classKey);
        const targetProgramAcct = await this.connection.getAccountInfo(targetProgramId);
        if (!targetProgramAcct) {
            throw new Error('Program not found');
        }
        const targetProgramDataKey = new web3_js_1.PublicKey(targetProgramAcct.data.slice(3));
        const ix = instruction_1.deleteMetadataEntryIx(this.programMetadataKey, classKey, nameKey, targetProgramId, targetProgramDataKey, targetProgramAuthorityKey, refundKey, this.nameServiceKey);
        return ix;
    }
    getHashedName(name) {
        let input = exports.NS_HASH_PREFIX + name;
        let buffer = crypto_1.createHash('sha256').update(input, 'utf8').digest();
        return buffer;
    }
    async getClassKey(targetProgramId) {
        const [classKey] = await web3_js_1.PublicKey.findProgramAddress([
            Buffer.from(exports.METADATA_PREFIX),
            targetProgramId.toBuffer()
        ], this.programMetadataKey);
        return classKey;
    }
    async getNameKey(hashedName, classKey) {
        const [nameKey] = await web3_js_1.PublicKey.findProgramAddress([
            hashedName,
            classKey.toBuffer(),
            Buffer.alloc(32)
        ], this.nameServiceKey);
        return nameKey;
    }
}
exports.ProgramMetadata = ProgramMetadata;
//# sourceMappingURL=bindings.js.map