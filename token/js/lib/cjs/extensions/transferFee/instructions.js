"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.decodeHarvestWithheldTokensToMintInstructionUnchecked = exports.decodeHarvestWithheldTokensToMintInstruction = exports.createHarvestWithheldTokensToMintInstruction = exports.harvestWithheldTokensToMintInstructionData = exports.decodeWithdrawWithheldTokensFromAccountsInstructionUnchecked = exports.decodeWithdrawWithheldTokensFromAccountsInstruction = exports.createWithdrawWithheldTokensFromAccountsInstruction = exports.withdrawWithheldTokensFromAccountsInstructionData = exports.decodeWithdrawWithheldTokensFromMintInstructionUnchecked = exports.decodeWithdrawWithheldTokensFromMintInstruction = exports.createWithdrawWithheldTokensFromMintInstruction = exports.withdrawWithheldTokensFromMintInstructionData = exports.decodeTransferCheckedWithFeeInstructionUnchecked = exports.decodeTransferCheckedWithFeeInstruction = exports.createTransferCheckedWithFeeInstruction = exports.transferCheckedWithFeeInstructionData = exports.decodeInitializeTransferFeeConfigInstructionUnchecked = exports.decodeInitializeTransferFeeConfigInstruction = exports.createInitializeTransferFeeConfigInstruction = exports.initializeTransferFeeConfigInstructionData = exports.TransferFeeInstruction = void 0;
const buffer_layout_1 = require("@solana/buffer-layout");
const buffer_layout_utils_1 = require("@solana/buffer-layout-utils");
const web3_js_1 = require("@solana/web3.js");
const constants_js_1 = require("../../constants.js");
const errors_js_1 = require("../../errors.js");
const internal_js_1 = require("../../instructions/internal.js");
const types_js_1 = require("../../instructions/types.js");
var TransferFeeInstruction;
(function (TransferFeeInstruction) {
    TransferFeeInstruction[TransferFeeInstruction["InitializeTransferFeeConfig"] = 0] = "InitializeTransferFeeConfig";
    TransferFeeInstruction[TransferFeeInstruction["TransferCheckedWithFee"] = 1] = "TransferCheckedWithFee";
    TransferFeeInstruction[TransferFeeInstruction["WithdrawWithheldTokensFromMint"] = 2] = "WithdrawWithheldTokensFromMint";
    TransferFeeInstruction[TransferFeeInstruction["WithdrawWithheldTokensFromAccounts"] = 3] = "WithdrawWithheldTokensFromAccounts";
    TransferFeeInstruction[TransferFeeInstruction["HarvestWithheldTokensToMint"] = 4] = "HarvestWithheldTokensToMint";
    TransferFeeInstruction[TransferFeeInstruction["SetTransferFee"] = 5] = "SetTransferFee";
})(TransferFeeInstruction = exports.TransferFeeInstruction || (exports.TransferFeeInstruction = {}));
/** TODO: docs */
exports.initializeTransferFeeConfigInstructionData = (0, buffer_layout_1.struct)([
    (0, buffer_layout_1.u8)('instruction'),
    (0, buffer_layout_1.u8)('transferFeeInstruction'),
    (0, buffer_layout_1.u8)('transferFeeConfigAuthorityOption'),
    (0, buffer_layout_utils_1.publicKey)('transferFeeConfigAuthority'),
    (0, buffer_layout_1.u8)('withdrawWithheldAuthorityOption'),
    (0, buffer_layout_utils_1.publicKey)('withdrawWithheldAuthority'),
    (0, buffer_layout_1.u16)('transferFeeBasisPoints'),
    (0, buffer_layout_utils_1.u64)('maximumFee'),
]);
/**
 * Construct an InitializeTransferFeeConfig instruction
 *
 * @param mint            Token mint account
 * @param transferFeeConfigAuthority  Optional authority that can update the fees
 * @param withdrawWithheldAuthority Optional authority that can withdraw fees
 * @param transferFeeBasisPoints Amount of transfer collected as fees, expressed as basis points of the transfer amount
 * @param maximumFee        Maximum fee assessed on transfers
 * @param programId       SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
function createInitializeTransferFeeConfigInstruction(mint, transferFeeConfigAuthority, withdrawWithheldAuthority, transferFeeBasisPoints, maximumFee, programId = constants_js_1.TOKEN_2022_PROGRAM_ID) {
    if (!(0, constants_js_1.programSupportsExtensions)(programId)) {
        throw new errors_js_1.TokenUnsupportedInstructionError();
    }
    const keys = [{ pubkey: mint, isSigner: false, isWritable: true }];
    const data = Buffer.alloc(exports.initializeTransferFeeConfigInstructionData.span);
    exports.initializeTransferFeeConfigInstructionData.encode({
        instruction: types_js_1.TokenInstruction.TransferFeeExtension,
        transferFeeInstruction: TransferFeeInstruction.InitializeTransferFeeConfig,
        transferFeeConfigAuthorityOption: transferFeeConfigAuthority ? 1 : 0,
        transferFeeConfigAuthority: transferFeeConfigAuthority || new web3_js_1.PublicKey(0),
        withdrawWithheldAuthorityOption: withdrawWithheldAuthority ? 1 : 0,
        withdrawWithheldAuthority: withdrawWithheldAuthority || new web3_js_1.PublicKey(0),
        transferFeeBasisPoints: transferFeeBasisPoints,
        maximumFee: maximumFee,
    }, data);
    return new web3_js_1.TransactionInstruction({ keys, programId, data });
}
exports.createInitializeTransferFeeConfigInstruction = createInitializeTransferFeeConfigInstruction;
/**
 * Decode an InitializeTransferFeeConfig instruction and validate it
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded, valid instruction
 */
function decodeInitializeTransferFeeConfigInstruction(instruction, programId) {
    if (!instruction.programId.equals(programId))
        throw new errors_js_1.TokenInvalidInstructionProgramError();
    if (instruction.data.length !== exports.initializeTransferFeeConfigInstructionData.span)
        throw new errors_js_1.TokenInvalidInstructionDataError();
    const { keys: { mint }, data, } = decodeInitializeTransferFeeConfigInstructionUnchecked(instruction);
    if (data.instruction !== types_js_1.TokenInstruction.TransferFeeExtension ||
        data.transferFeeInstruction !== TransferFeeInstruction.InitializeTransferFeeConfig)
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
exports.decodeInitializeTransferFeeConfigInstruction = decodeInitializeTransferFeeConfigInstruction;
/**
 * Decode an InitializeTransferFeeConfig instruction without validating it
 *
 * @param instruction Transaction instruction to decode
 *
 * @return Decoded, non-validated instruction
 */
function decodeInitializeTransferFeeConfigInstructionUnchecked({ programId, keys: [mint], data, }) {
    const { instruction, transferFeeInstruction, transferFeeConfigAuthorityOption, transferFeeConfigAuthority, withdrawWithheldAuthorityOption, withdrawWithheldAuthority, transferFeeBasisPoints, maximumFee, } = exports.initializeTransferFeeConfigInstructionData.decode(data);
    return {
        programId,
        keys: {
            mint,
        },
        data: {
            instruction,
            transferFeeInstruction,
            transferFeeConfigAuthority: transferFeeConfigAuthorityOption ? transferFeeConfigAuthority : null,
            withdrawWithheldAuthority: withdrawWithheldAuthorityOption ? withdrawWithheldAuthority : null,
            transferFeeBasisPoints,
            maximumFee,
        },
    };
}
exports.decodeInitializeTransferFeeConfigInstructionUnchecked = decodeInitializeTransferFeeConfigInstructionUnchecked;
exports.transferCheckedWithFeeInstructionData = (0, buffer_layout_1.struct)([
    (0, buffer_layout_1.u8)('instruction'),
    (0, buffer_layout_1.u8)('transferFeeInstruction'),
    (0, buffer_layout_utils_1.u64)('amount'),
    (0, buffer_layout_1.u8)('decimals'),
    (0, buffer_layout_utils_1.u64)('fee'),
]);
/**
 * Construct an TransferCheckedWithFee instruction
 *
 * @param source          The source account
 * @param mint            The token mint
 * @param destination     The destination account
 * @param authority       The source account's owner/delegate
 * @param signers         The signer account(s)
 * @param amount          The amount of tokens to transfer
 * @param decimals        The expected number of base 10 digits to the right of the decimal place
 * @param fee             The expected fee assesed on this transfer, calculated off-chain based on the transferFeeBasisPoints and maximumFee of the mint.
 * @param programId       SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
function createTransferCheckedWithFeeInstruction(source, mint, destination, authority, amount, decimals, fee, multiSigners = [], programId = constants_js_1.TOKEN_2022_PROGRAM_ID) {
    if (!(0, constants_js_1.programSupportsExtensions)(programId)) {
        throw new errors_js_1.TokenUnsupportedInstructionError();
    }
    const data = Buffer.alloc(exports.transferCheckedWithFeeInstructionData.span);
    exports.transferCheckedWithFeeInstructionData.encode({
        instruction: types_js_1.TokenInstruction.TransferFeeExtension,
        transferFeeInstruction: TransferFeeInstruction.TransferCheckedWithFee,
        amount,
        decimals,
        fee,
    }, data);
    const keys = (0, internal_js_1.addSigners)([
        { pubkey: source, isSigner: false, isWritable: true },
        { pubkey: mint, isSigner: false, isWritable: false },
        { pubkey: destination, isSigner: false, isWritable: true },
    ], authority, multiSigners);
    return new web3_js_1.TransactionInstruction({ keys, programId, data });
}
exports.createTransferCheckedWithFeeInstruction = createTransferCheckedWithFeeInstruction;
/**
 * Decode a TransferCheckedWithFee instruction and validate it
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded, valid instruction
 */
function decodeTransferCheckedWithFeeInstruction(instruction, programId) {
    if (!instruction.programId.equals(programId))
        throw new errors_js_1.TokenInvalidInstructionProgramError();
    if (instruction.data.length !== exports.transferCheckedWithFeeInstructionData.span)
        throw new errors_js_1.TokenInvalidInstructionDataError();
    const { keys: { source, mint, destination, authority, signers }, data, } = decodeTransferCheckedWithFeeInstructionUnchecked(instruction);
    if (data.instruction !== types_js_1.TokenInstruction.TransferFeeExtension ||
        data.transferFeeInstruction !== TransferFeeInstruction.TransferCheckedWithFee)
        throw new errors_js_1.TokenInvalidInstructionTypeError();
    if (!mint)
        throw new errors_js_1.TokenInvalidInstructionKeysError();
    return {
        programId,
        keys: {
            source,
            mint,
            destination,
            authority,
            signers: signers ? signers : null,
        },
        data,
    };
}
exports.decodeTransferCheckedWithFeeInstruction = decodeTransferCheckedWithFeeInstruction;
/**
 * Decode a TransferCheckedWithFees instruction without validating it
 *
 * @param instruction Transaction instruction to decode
 *
 * @return Decoded, non-validated instruction
 */
function decodeTransferCheckedWithFeeInstructionUnchecked({ programId, keys: [source, mint, destination, authority, ...signers], data, }) {
    const { instruction, transferFeeInstruction, amount, decimals, fee } = exports.transferCheckedWithFeeInstructionData.decode(data);
    return {
        programId,
        keys: {
            source,
            mint,
            destination,
            authority,
            signers,
        },
        data: {
            instruction,
            transferFeeInstruction,
            amount,
            decimals,
            fee,
        },
    };
}
exports.decodeTransferCheckedWithFeeInstructionUnchecked = decodeTransferCheckedWithFeeInstructionUnchecked;
exports.withdrawWithheldTokensFromMintInstructionData = (0, buffer_layout_1.struct)([
    (0, buffer_layout_1.u8)('instruction'),
    (0, buffer_layout_1.u8)('transferFeeInstruction'),
]);
/**
 * Construct a WithdrawWithheldTokensFromMint instruction
 *
 * @param mint              The token mint
 * @param destination       The destination account
 * @param authority         The source account's owner/delegate
 * @param signers           The signer account(s)
 * @param programID         SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
function createWithdrawWithheldTokensFromMintInstruction(mint, destination, authority, signers = [], programId = constants_js_1.TOKEN_2022_PROGRAM_ID) {
    if (!(0, constants_js_1.programSupportsExtensions)(programId)) {
        throw new errors_js_1.TokenUnsupportedInstructionError();
    }
    const data = Buffer.alloc(exports.withdrawWithheldTokensFromMintInstructionData.span);
    exports.withdrawWithheldTokensFromMintInstructionData.encode({
        instruction: types_js_1.TokenInstruction.TransferFeeExtension,
        transferFeeInstruction: TransferFeeInstruction.WithdrawWithheldTokensFromMint,
    }, data);
    const keys = (0, internal_js_1.addSigners)([
        { pubkey: mint, isSigner: false, isWritable: true },
        { pubkey: destination, isSigner: false, isWritable: true },
    ], authority, signers);
    return new web3_js_1.TransactionInstruction({ keys, programId, data });
}
exports.createWithdrawWithheldTokensFromMintInstruction = createWithdrawWithheldTokensFromMintInstruction;
/**
 * Decode a WithdrawWithheldTokensFromMint instruction and validate it
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded, valid instruction
 */
function decodeWithdrawWithheldTokensFromMintInstruction(instruction, programId) {
    if (!instruction.programId.equals(programId))
        throw new errors_js_1.TokenInvalidInstructionProgramError();
    if (instruction.data.length !== exports.withdrawWithheldTokensFromMintInstructionData.span)
        throw new errors_js_1.TokenInvalidInstructionDataError();
    const { keys: { mint, destination, authority, signers }, data, } = decodeWithdrawWithheldTokensFromMintInstructionUnchecked(instruction);
    if (data.instruction !== types_js_1.TokenInstruction.TransferFeeExtension ||
        data.transferFeeInstruction !== TransferFeeInstruction.WithdrawWithheldTokensFromMint)
        throw new errors_js_1.TokenInvalidInstructionTypeError();
    if (!mint)
        throw new errors_js_1.TokenInvalidInstructionKeysError();
    return {
        programId,
        keys: {
            mint,
            destination,
            authority,
            signers: signers ? signers : null,
        },
        data,
    };
}
exports.decodeWithdrawWithheldTokensFromMintInstruction = decodeWithdrawWithheldTokensFromMintInstruction;
/**
 * Decode a WithdrawWithheldTokensFromMint instruction without validating it
 *
 * @param instruction Transaction instruction to decode
 *
 * @return Decoded, non-validated instruction
 */
function decodeWithdrawWithheldTokensFromMintInstructionUnchecked({ programId, keys: [mint, destination, authority, ...signers], data, }) {
    const { instruction, transferFeeInstruction } = exports.withdrawWithheldTokensFromMintInstructionData.decode(data);
    return {
        programId,
        keys: {
            mint,
            destination,
            authority,
            signers,
        },
        data: {
            instruction,
            transferFeeInstruction,
        },
    };
}
exports.decodeWithdrawWithheldTokensFromMintInstructionUnchecked = decodeWithdrawWithheldTokensFromMintInstructionUnchecked;
exports.withdrawWithheldTokensFromAccountsInstructionData = (0, buffer_layout_1.struct)([
    (0, buffer_layout_1.u8)('instruction'),
    (0, buffer_layout_1.u8)('transferFeeInstruction'),
    (0, buffer_layout_1.u8)('numTokenAccounts'),
]);
/**
 * Construct a WithdrawWithheldTokensFromAccounts instruction
 *
 * @param mint              The token mint
 * @param destination       The destination account
 * @param authority         The source account's owner/delegate
 * @param signers           The signer account(s)
 * @param sources           The source accounts to withdraw from
 * @param programID         SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
function createWithdrawWithheldTokensFromAccountsInstruction(mint, destination, authority, signers, sources, programId = constants_js_1.TOKEN_2022_PROGRAM_ID) {
    if (!(0, constants_js_1.programSupportsExtensions)(programId)) {
        throw new errors_js_1.TokenUnsupportedInstructionError();
    }
    const data = Buffer.alloc(exports.withdrawWithheldTokensFromAccountsInstructionData.span);
    exports.withdrawWithheldTokensFromAccountsInstructionData.encode({
        instruction: types_js_1.TokenInstruction.TransferFeeExtension,
        transferFeeInstruction: TransferFeeInstruction.WithdrawWithheldTokensFromAccounts,
        numTokenAccounts: sources.length,
    }, data);
    const keys = (0, internal_js_1.addSigners)([
        { pubkey: mint, isSigner: false, isWritable: true },
        { pubkey: destination, isSigner: false, isWritable: true },
    ], authority, signers);
    for (const source of sources) {
        keys.push({ pubkey: source, isSigner: false, isWritable: true });
    }
    return new web3_js_1.TransactionInstruction({ keys, programId, data });
}
exports.createWithdrawWithheldTokensFromAccountsInstruction = createWithdrawWithheldTokensFromAccountsInstruction;
/**
 * Decode a WithdrawWithheldTokensFromAccounts instruction and validate it
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded, valid instruction
 */
function decodeWithdrawWithheldTokensFromAccountsInstruction(instruction, programId) {
    if (!instruction.programId.equals(programId))
        throw new errors_js_1.TokenInvalidInstructionProgramError();
    if (instruction.data.length !== exports.withdrawWithheldTokensFromAccountsInstructionData.span)
        throw new errors_js_1.TokenInvalidInstructionDataError();
    const { keys: { mint, destination, authority, signers, sources }, data, } = decodeWithdrawWithheldTokensFromAccountsInstructionUnchecked(instruction);
    if (data.instruction !== types_js_1.TokenInstruction.TransferFeeExtension ||
        data.transferFeeInstruction !== TransferFeeInstruction.WithdrawWithheldTokensFromAccounts)
        throw new errors_js_1.TokenInvalidInstructionTypeError();
    if (!mint)
        throw new errors_js_1.TokenInvalidInstructionKeysError();
    return {
        programId,
        keys: {
            mint,
            destination,
            authority,
            signers: signers ? signers : null,
            sources: sources ? sources : null,
        },
        data,
    };
}
exports.decodeWithdrawWithheldTokensFromAccountsInstruction = decodeWithdrawWithheldTokensFromAccountsInstruction;
/**
 * Decode a WithdrawWithheldTokensFromAccount instruction without validating it
 *
 * @param instruction Transaction instruction to decode
 *
 * @return Decoded, non-validated instruction
 */
function decodeWithdrawWithheldTokensFromAccountsInstructionUnchecked({ programId, keys, data, }) {
    const { instruction, transferFeeInstruction, numTokenAccounts } = exports.withdrawWithheldTokensFromAccountsInstructionData.decode(data);
    const [mint, destination, authority, signers, sources] = [
        keys[0],
        keys[1],
        keys[2],
        keys.slice(3, 3 + numTokenAccounts),
        keys.slice(-1 * numTokenAccounts),
    ];
    return {
        programId,
        keys: {
            mint,
            destination,
            authority,
            signers,
            sources,
        },
        data: {
            instruction,
            transferFeeInstruction,
            numTokenAccounts,
        },
    };
}
exports.decodeWithdrawWithheldTokensFromAccountsInstructionUnchecked = decodeWithdrawWithheldTokensFromAccountsInstructionUnchecked;
exports.harvestWithheldTokensToMintInstructionData = (0, buffer_layout_1.struct)([
    (0, buffer_layout_1.u8)('instruction'),
    (0, buffer_layout_1.u8)('transferFeeInstruction'),
]);
/**
 * Construct a HarvestWithheldTokensToMint instruction
 *
 * @param mint              The token mint
 * @param sources           The source accounts to withdraw from
 * @param programID         SPL Token program account
 *
 * @return Instruction to add to a transaction
 */
function createHarvestWithheldTokensToMintInstruction(mint, sources, programId = constants_js_1.TOKEN_2022_PROGRAM_ID) {
    if (!(0, constants_js_1.programSupportsExtensions)(programId)) {
        throw new errors_js_1.TokenUnsupportedInstructionError();
    }
    const data = Buffer.alloc(exports.harvestWithheldTokensToMintInstructionData.span);
    exports.harvestWithheldTokensToMintInstructionData.encode({
        instruction: types_js_1.TokenInstruction.TransferFeeExtension,
        transferFeeInstruction: TransferFeeInstruction.HarvestWithheldTokensToMint,
    }, data);
    const keys = [];
    keys.push({ pubkey: mint, isSigner: false, isWritable: true });
    for (const source of sources) {
        keys.push({ pubkey: source, isSigner: false, isWritable: true });
    }
    return new web3_js_1.TransactionInstruction({ keys, programId, data });
}
exports.createHarvestWithheldTokensToMintInstruction = createHarvestWithheldTokensToMintInstruction;
/**
 * Decode a HarvestWithheldTokensToMint instruction and validate it
 *
 * @param instruction Transaction instruction to decode
 * @param programId   SPL Token program account
 *
 * @return Decoded, valid instruction
 */
function decodeHarvestWithheldTokensToMintInstruction(instruction, programId) {
    if (!instruction.programId.equals(programId))
        throw new errors_js_1.TokenInvalidInstructionProgramError();
    if (instruction.data.length !== exports.harvestWithheldTokensToMintInstructionData.span)
        throw new errors_js_1.TokenInvalidInstructionDataError();
    const { keys: { mint, sources }, data, } = decodeHarvestWithheldTokensToMintInstructionUnchecked(instruction);
    if (data.instruction !== types_js_1.TokenInstruction.TransferFeeExtension ||
        data.transferFeeInstruction !== TransferFeeInstruction.HarvestWithheldTokensToMint)
        throw new errors_js_1.TokenInvalidInstructionTypeError();
    if (!mint)
        throw new errors_js_1.TokenInvalidInstructionKeysError();
    return {
        programId,
        keys: {
            mint,
            sources,
        },
        data,
    };
}
exports.decodeHarvestWithheldTokensToMintInstruction = decodeHarvestWithheldTokensToMintInstruction;
/**
 * Decode a HarvestWithheldTokensToMint instruction without validating it
 *
 * @param instruction Transaction instruction to decode
 *
 * @return Decoded, non-validated instruction
 */
function decodeHarvestWithheldTokensToMintInstructionUnchecked({ programId, keys: [mint, ...sources], data, }) {
    const { instruction, transferFeeInstruction } = exports.harvestWithheldTokensToMintInstructionData.decode(data);
    return {
        programId,
        keys: {
            mint,
            sources,
        },
        data: {
            instruction,
            transferFeeInstruction,
        },
    };
}
exports.decodeHarvestWithheldTokensToMintInstructionUnchecked = decodeHarvestWithheldTokensToMintInstructionUnchecked;
//# sourceMappingURL=instructions.js.map