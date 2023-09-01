"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.isUiamountToAmountInstruction = exports.isAmountToUiAmountInstruction = exports.isInitializeMint2Instruction = exports.isInitializeAccount3Instruction = exports.isSyncNativeInstruction = exports.isInitializeAccount2Instruction = exports.isBurnCheckedInstruction = exports.isMintToCheckedInstruction = exports.isApproveCheckedInstruction = exports.isTransferCheckedInstruction = exports.isThawAccountInstruction = exports.isFreezeAccountInstruction = exports.isCloseAccountInstruction = exports.isBurnInstruction = exports.isMintToInstruction = exports.isSetAuthorityInstruction = exports.isRevokeInstruction = exports.isApproveInstruction = exports.isTransferInstruction = exports.isInitializeMultisigInstruction = exports.isInitializeAccountInstruction = exports.isInitializeMintInstruction = exports.decodeInstruction = void 0;
const buffer_layout_1 = require("@solana/buffer-layout");
const constants_js_1 = require("../constants.js");
const errors_js_1 = require("../errors.js");
const amountToUiAmount_js_1 = require("./amountToUiAmount.js");
const approve_js_1 = require("./approve.js");
const approveChecked_js_1 = require("./approveChecked.js");
const burn_js_1 = require("./burn.js");
const burnChecked_js_1 = require("./burnChecked.js");
const closeAccount_js_1 = require("./closeAccount.js");
const freezeAccount_js_1 = require("./freezeAccount.js");
const initializeAccount_js_1 = require("./initializeAccount.js");
const initializeAccount2_js_1 = require("./initializeAccount2.js");
const initializeAccount3_js_1 = require("./initializeAccount3.js");
const initializeMint_js_1 = require("./initializeMint.js");
const initializeMint2_js_1 = require("./initializeMint2.js");
const initializeMultisig_js_1 = require("./initializeMultisig.js");
const mintTo_js_1 = require("./mintTo.js");
const mintToChecked_js_1 = require("./mintToChecked.js");
const revoke_js_1 = require("./revoke.js");
const setAuthority_js_1 = require("./setAuthority.js");
const syncNative_js_1 = require("./syncNative.js");
const thawAccount_js_1 = require("./thawAccount.js");
const transfer_js_1 = require("./transfer.js");
const transferChecked_js_1 = require("./transferChecked.js");
const types_js_1 = require("./types.js");
const uiAmountToAmount_js_1 = require("./uiAmountToAmount.js");
/** TODO: docs */
function decodeInstruction(instruction, programId = constants_js_1.TOKEN_PROGRAM_ID) {
    if (!instruction.data.length)
        throw new errors_js_1.TokenInvalidInstructionDataError();
    const type = (0, buffer_layout_1.u8)().decode(instruction.data);
    if (type === types_js_1.TokenInstruction.InitializeMint)
        return (0, initializeMint_js_1.decodeInitializeMintInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.InitializeAccount)
        return (0, initializeAccount_js_1.decodeInitializeAccountInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.InitializeMultisig)
        return (0, initializeMultisig_js_1.decodeInitializeMultisigInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.Transfer)
        return (0, transfer_js_1.decodeTransferInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.Approve)
        return (0, approve_js_1.decodeApproveInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.Revoke)
        return (0, revoke_js_1.decodeRevokeInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.SetAuthority)
        return (0, setAuthority_js_1.decodeSetAuthorityInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.MintTo)
        return (0, mintTo_js_1.decodeMintToInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.Burn)
        return (0, burn_js_1.decodeBurnInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.CloseAccount)
        return (0, closeAccount_js_1.decodeCloseAccountInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.FreezeAccount)
        return (0, freezeAccount_js_1.decodeFreezeAccountInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.ThawAccount)
        return (0, thawAccount_js_1.decodeThawAccountInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.TransferChecked)
        return (0, transferChecked_js_1.decodeTransferCheckedInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.ApproveChecked)
        return (0, approveChecked_js_1.decodeApproveCheckedInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.MintToChecked)
        return (0, mintToChecked_js_1.decodeMintToCheckedInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.BurnChecked)
        return (0, burnChecked_js_1.decodeBurnCheckedInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.InitializeAccount2)
        return (0, initializeAccount2_js_1.decodeInitializeAccount2Instruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.SyncNative)
        return (0, syncNative_js_1.decodeSyncNativeInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.InitializeAccount3)
        return (0, initializeAccount3_js_1.decodeInitializeAccount3Instruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.InitializeMint2)
        return (0, initializeMint2_js_1.decodeInitializeMint2Instruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.AmountToUiAmount)
        return (0, amountToUiAmount_js_1.decodeAmountToUiAmountInstruction)(instruction, programId);
    if (type === types_js_1.TokenInstruction.UiAmountToAmount)
        return (0, uiAmountToAmount_js_1.decodeUiAmountToAmountInstruction)(instruction, programId);
    // TODO: implement
    if (type === types_js_1.TokenInstruction.InitializeMultisig2)
        throw new errors_js_1.TokenInvalidInstructionTypeError();
    throw new errors_js_1.TokenInvalidInstructionTypeError();
}
exports.decodeInstruction = decodeInstruction;
/** TODO: docs */
function isInitializeMintInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.InitializeMint;
}
exports.isInitializeMintInstruction = isInitializeMintInstruction;
/** TODO: docs */
function isInitializeAccountInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.InitializeAccount;
}
exports.isInitializeAccountInstruction = isInitializeAccountInstruction;
/** TODO: docs */
function isInitializeMultisigInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.InitializeMultisig;
}
exports.isInitializeMultisigInstruction = isInitializeMultisigInstruction;
/** TODO: docs */
function isTransferInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.Transfer;
}
exports.isTransferInstruction = isTransferInstruction;
/** TODO: docs */
function isApproveInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.Approve;
}
exports.isApproveInstruction = isApproveInstruction;
/** TODO: docs */
function isRevokeInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.Revoke;
}
exports.isRevokeInstruction = isRevokeInstruction;
/** TODO: docs */
function isSetAuthorityInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.SetAuthority;
}
exports.isSetAuthorityInstruction = isSetAuthorityInstruction;
/** TODO: docs */
function isMintToInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.MintTo;
}
exports.isMintToInstruction = isMintToInstruction;
/** TODO: docs */
function isBurnInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.Burn;
}
exports.isBurnInstruction = isBurnInstruction;
/** TODO: docs */
function isCloseAccountInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.CloseAccount;
}
exports.isCloseAccountInstruction = isCloseAccountInstruction;
/** TODO: docs */
function isFreezeAccountInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.FreezeAccount;
}
exports.isFreezeAccountInstruction = isFreezeAccountInstruction;
/** TODO: docs */
function isThawAccountInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.ThawAccount;
}
exports.isThawAccountInstruction = isThawAccountInstruction;
/** TODO: docs */
function isTransferCheckedInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.TransferChecked;
}
exports.isTransferCheckedInstruction = isTransferCheckedInstruction;
/** TODO: docs */
function isApproveCheckedInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.ApproveChecked;
}
exports.isApproveCheckedInstruction = isApproveCheckedInstruction;
/** TODO: docs */
function isMintToCheckedInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.MintToChecked;
}
exports.isMintToCheckedInstruction = isMintToCheckedInstruction;
/** TODO: docs */
function isBurnCheckedInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.BurnChecked;
}
exports.isBurnCheckedInstruction = isBurnCheckedInstruction;
/** TODO: docs */
function isInitializeAccount2Instruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.InitializeAccount2;
}
exports.isInitializeAccount2Instruction = isInitializeAccount2Instruction;
/** TODO: docs */
function isSyncNativeInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.SyncNative;
}
exports.isSyncNativeInstruction = isSyncNativeInstruction;
/** TODO: docs */
function isInitializeAccount3Instruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.InitializeAccount3;
}
exports.isInitializeAccount3Instruction = isInitializeAccount3Instruction;
/** TODO: docs, implement */
// export function isInitializeMultisig2Instruction(
//     decoded: DecodedInstruction
// ): decoded is DecodedInitializeMultisig2Instruction {
//     return decoded.data.instruction === TokenInstruction.InitializeMultisig2;
// }
/** TODO: docs */
function isInitializeMint2Instruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.InitializeMint2;
}
exports.isInitializeMint2Instruction = isInitializeMint2Instruction;
/** TODO: docs */
function isAmountToUiAmountInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.AmountToUiAmount;
}
exports.isAmountToUiAmountInstruction = isAmountToUiAmountInstruction;
/** TODO: docs */
function isUiamountToAmountInstruction(decoded) {
    return decoded.data.instruction === types_js_1.TokenInstruction.UiAmountToAmount;
}
exports.isUiamountToAmountInstruction = isUiamountToAmountInstruction;
//# sourceMappingURL=decode.js.map