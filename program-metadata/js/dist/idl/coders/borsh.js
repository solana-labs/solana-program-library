"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Borsh = void 0;
const helpers_1 = require("../../program/util/helpers");
const coder_1 = require("../coder");
class Borsh extends coder_1.Coder {
    getInstruction(index) {
        if (index > this.idl.instructions.length) {
            throw new Error(`Instruction at index ${index} not found`);
        }
        return this.idl.instructions[index];
    }
    getInstructionIndex(data) {
        return data.readUInt8(0);
    }
    getAccounts(idlInstruction, transactionInstruction) {
        const accounts = Array.isArray(idlInstruction.accounts) ? idlInstruction.accounts : [idlInstruction.accounts];
        return accounts.map((def, i) => {
            if (!transactionInstruction.keys[i]) {
                return {
                    message: `Account ${i} is missing`
                };
            }
            return Object.assign({
                formattedName: def.name,
                pubkey: transactionInstruction.keys[i].pubkey
            }, def);
        });
    }
    getArguments(idlInstruction, transactionInstruction) {
    }
    buildSchema(idlInstruction) {
        const schema = {
            kind: "struct",
            fields: [
                ["instruction", "u8"]
            ]
        };
        return schema;
    }
    buildDataObject(idlInstruction) {
    }
    decodeInstruction(instruction) {
        const index = this.getInstructionIndex(instruction.data);
        const idlIx = this.getInstruction(index);
        const programId = instruction.programId;
        const name = idlIx.name;
        const formattedName = helpers_1.startCase(name);
        const accounts = this.getAccounts(idlIx, instruction);
        const args = this.getArguments(idlIx, instruction);
        return {
            name,
            formattedName,
            programId,
            accounts,
            args
        };
    }
}
exports.Borsh = Borsh;
//# sourceMappingURL=borsh.js.map