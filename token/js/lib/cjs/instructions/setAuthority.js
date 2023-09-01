"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.decodeSetAuthorityInstructionUnchecked = exports.decodeSetAuthorityInstruction = exports.createSetAuthorityInstruction = exports.setAuthorityInstructionData = exports.AuthorityType = void 0;
const buffer_layout_1 = require("@solana/buffer-layout");
const buffer_layout_utils_1 = require("@solana/buffer-layout-utils");
const web3_js_1 = require("@solana/web3.js");
const constants_js_1 = require("../constants.js");
const errors_js_1 = require("../errors.js");
const internal_js_1 = require("./internal.js");
const types_js_1 = require("./types.js");
/** Authority types defined by the program */
var AuthorityType;
(function (AuthorityType) {
    AuthorityType[AuthorityType["MintTokens"] = 0] = "MintTokens";
    AuthorityType[AuthorityType["FreezeAccount"] = 1] = "FreezeAccount";
    AuthorityType[AuthorityType["AccountOwner"] = 2] = "AccountOwner";
    AuthorityType[AuthorityType["CloseAccount"] = 3] = "CloseAccount";
})(AuthorityType = exports.AuthorityType || (exports.AuthorityType = {}));
/** TODO: docs */
exports.setAuthorityInstructionData = (0, buffer_layout_1.struct)([
    (0, buffer_layout_1.u8)('instruction'),
    (0, buffer_layout_1.u8)('authorityType'),
    (0, buffer_layout_1.u8)('newAuthorityOption'),
    (0, buffer_layout_utils_1.publicKey)('newAuthority'),
]);
/**
 * Construct a SetAuthority instruction
 *
 * @param account          Address of the token account
 * @param currentAuthority Current authority of the specified type
 * @param authorityType    Type of authority to set
 * @param newAuthority     New authority of the account
 * @param multiSigners     Signing accounts if `currentAuthority` is a multisig
 * @param programId        SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
function createSetAuthorityInstruction(account, currentAuthority, authorityType, newAuthority, multiSigners = [], programId = constants_js_1.TOKEN_PROGRAM_ID) {
    const keys = (0, internal_js_1.addSigners)([{ pubkey: account, isSigner: false, isWritable: true }], currentAuthority, multiSigners);
    const data = Buffer.alloc(exports.setAuthorityInstructionData.span);
    exports.setAuthorityInstructionData.encode({
        instruction: types_js_1.TokenInstruction.SetAuthority,
        authorityType,
        newAuthorityOption: newAuthority ? 1 : 0,
        newAuthority: newAuthority || new web3_js_1.PublicKey(0),
    }, data);
    return new web3_js_1.TransactionInstruction({ keys, programId, data });
}
exports.createSetAuthorityInstruction = createSetAuthorityInstruction;
/**
 * Decode a SetAuthority instruction and validate it
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded, valid instruction
 */
function decodeSetAuthorityInstruction(instruction, programId = constants_js_1.TOKEN_PROGRAM_ID) {
    if (!instruction.programId.equals(programId))
        throw new errors_js_1.TokenInvalidInstructionProgramError();
    if (instruction.data.length !== exports.setAuthorityInstructionData.span)
        throw new errors_js_1.TokenInvalidInstructionDataError();
    const { keys: { account, currentAuthority, multiSigners }, data, } = decodeSetAuthorityInstructionUnchecked(instruction);
    if (data.instruction !== types_js_1.TokenInstruction.SetAuthority)
        throw new errors_js_1.TokenInvalidInstructionTypeError();
    if (!account || !currentAuthority)
        throw new errors_js_1.TokenInvalidInstructionKeysError();
    // TODO: key checks?
    return {
        programId,
        keys: {
            account,
            currentAuthority,
            multiSigners,
        },
        data,
    };
}
exports.decodeSetAuthorityInstruction = decodeSetAuthorityInstruction;
/**
 * Decode a SetAuthority instruction without validating it
 *
 * @param instruction Transaction instruction to decode
 *
 * @return Decoded, non-validated instruction
 */
function decodeSetAuthorityInstructionUnchecked({ programId, keys: [account, currentAuthority, ...multiSigners], data, }) {
    const { instruction, authorityType, newAuthorityOption, newAuthority } = exports.setAuthorityInstructionData.decode(data);
    return {
        programId,
        keys: {
            account,
            currentAuthority,
            multiSigners,
        },
        data: {
            instruction,
            authorityType,
            newAuthority: newAuthorityOption ? newAuthority : null,
        },
    };
}
exports.decodeSetAuthorityInstructionUnchecked = decodeSetAuthorityInstructionUnchecked;
//# sourceMappingURL=setAuthority.js.map