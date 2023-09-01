"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.decodeInitializeMint2InstructionUnchecked = exports.decodeInitializeMint2Instruction = exports.createInitializeMint2Instruction = exports.initializeMint2InstructionData = void 0;
const buffer_layout_1 = require("@solana/buffer-layout");
const buffer_layout_utils_1 = require("@solana/buffer-layout-utils");
const web3_js_1 = require("@solana/web3.js");
const constants_js_1 = require("../constants.js");
const errors_js_1 = require("../errors.js");
const types_js_1 = require("./types.js");
/** TODO: docs */
exports.initializeMint2InstructionData = (0, buffer_layout_1.struct)([
    (0, buffer_layout_1.u8)('instruction'),
    (0, buffer_layout_1.u8)('decimals'),
    (0, buffer_layout_utils_1.publicKey)('mintAuthority'),
    (0, buffer_layout_1.u8)('freezeAuthorityOption'),
    (0, buffer_layout_utils_1.publicKey)('freezeAuthority'),
]);
/**
 * Construct an InitializeMint2 instruction
 *
 * @param mint            Token mint account
 * @param decimals        Number of decimals in token account amounts
 * @param mintAuthority   Minting authority
 * @param freezeAuthority Optional authority that can freeze token accounts
 * @param programId       SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
function createInitializeMint2Instruction(mint, decimals, mintAuthority, freezeAuthority, programId = constants_js_1.TOKEN_PROGRAM_ID) {
    const keys = [{ pubkey: mint, isSigner: false, isWritable: true }];
    const data = Buffer.alloc(exports.initializeMint2InstructionData.span);
    exports.initializeMint2InstructionData.encode({
        instruction: types_js_1.TokenInstruction.InitializeMint2,
        decimals,
        mintAuthority,
        freezeAuthorityOption: freezeAuthority ? 1 : 0,
        freezeAuthority: freezeAuthority || new web3_js_1.PublicKey(0),
    }, data);
    return new web3_js_1.TransactionInstruction({ keys, programId, data });
}
exports.createInitializeMint2Instruction = createInitializeMint2Instruction;
/**
 * Decode an InitializeMint2 instruction and validate it
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded, valid instruction
 */
function decodeInitializeMint2Instruction(instruction, programId = constants_js_1.TOKEN_PROGRAM_ID) {
    if (!instruction.programId.equals(programId))
        throw new errors_js_1.TokenInvalidInstructionProgramError();
    if (instruction.data.length !== exports.initializeMint2InstructionData.span)
        throw new errors_js_1.TokenInvalidInstructionDataError();
    const { keys: { mint }, data, } = decodeInitializeMint2InstructionUnchecked(instruction);
    if (data.instruction !== types_js_1.TokenInstruction.InitializeMint2)
        throw new errors_js_1.TokenInvalidInstructionTypeError();
    if (!mint)
        throw new errors_js_1.TokenInvalidInstructionKeysError();
    return {
        programId,
        keys: {
            mint,
        },
        data,
    };
}
exports.decodeInitializeMint2Instruction = decodeInitializeMint2Instruction;
/**
 * Decode an InitializeMint2 instruction without validating it
 *
 * @param instruction Transaction instruction to decode
 *
 * @return Decoded, non-validated instruction
 */
function decodeInitializeMint2InstructionUnchecked({ programId, keys: [mint], data, }) {
    const { instruction, decimals, mintAuthority, freezeAuthorityOption, freezeAuthority } = exports.initializeMint2InstructionData.decode(data);
    return {
        programId,
        keys: {
            mint,
        },
        data: {
            instruction,
            decimals,
            mintAuthority,
            freezeAuthority: freezeAuthorityOption ? freezeAuthority : null,
        },
    };
}
exports.decodeInitializeMint2InstructionUnchecked = decodeInitializeMint2InstructionUnchecked;
//# sourceMappingURL=initializeMint2.js.map