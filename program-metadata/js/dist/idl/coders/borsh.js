"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    Object.defineProperty(o, k2, { enumerable: true, get: function() { return m[k]; } });
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.Borsh = void 0;
const helpers_1 = require("../../program/util/helpers");
const coder_1 = require("../coder");
const idl_borsh_1 = require("../util/idl-borsh");
const borsh = __importStar(require("@project-serum/borsh"));
const camelcase_1 = __importDefault(require("camelcase"));
class Borsh extends coder_1.Coder {
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
            args,
        };
    }
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
        const accounts = Array.isArray(idlInstruction.accounts)
            ? idlInstruction.accounts
            : [idlInstruction.accounts];
        return accounts.map((def, i) => {
            if (!transactionInstruction.keys[i]) {
                return {
                    message: `Account ${i} is missing`,
                };
            }
            return Object.assign({
                formattedName: helpers_1.startCase(def.name),
                pubkey: transactionInstruction.keys[i].pubkey,
            }, def);
        });
    }
    getArguments(idlInstruction, transactionInstruction) {
        const coder = this.buildCoder(idlInstruction);
        const decoded = coder.decode(transactionInstruction.data.slice(1)); // skip over enum header
        return idlInstruction.args.map((field) => {
            const name = camelcase_1.default(field.name);
            if (!(name in decoded)) {
                return {
                    message: `Field ${name} is missing`,
                };
            }
            return Object.assign({
                formattedName: helpers_1.startCase(field.name),
                value: decoded[name],
            }, field);
        });
    }
    buildCoder(idlInstruction) {
        const fieldLayouts = idlInstruction.args.map((arg) => idl_borsh_1.fieldLayout(arg, this.idl.types));
        return borsh.struct(fieldLayouts);
    }
}
exports.Borsh = Borsh;
//# sourceMappingURL=borsh.js.map