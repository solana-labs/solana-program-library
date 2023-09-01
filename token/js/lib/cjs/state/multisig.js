"use strict";
var __awaiter = (this && this.__awaiter) || function (thisArg, _arguments, P, generator) {
    function adopt(value) { return value instanceof P ? value : new P(function (resolve) { resolve(value); }); }
    return new (P || (P = Promise))(function (resolve, reject) {
        function fulfilled(value) { try { step(generator.next(value)); } catch (e) { reject(e); } }
        function rejected(value) { try { step(generator["throw"](value)); } catch (e) { reject(e); } }
        function step(result) { result.done ? resolve(result.value) : adopt(result.value).then(fulfilled, rejected); }
        step((generator = generator.apply(thisArg, _arguments || [])).next());
    });
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.getMinimumBalanceForRentExemptMultisig = exports.unpackMultisig = exports.getMultisig = exports.MULTISIG_SIZE = exports.MultisigLayout = void 0;
const buffer_layout_1 = require("@solana/buffer-layout");
const buffer_layout_utils_1 = require("@solana/buffer-layout-utils");
const constants_js_1 = require("../constants.js");
const errors_js_1 = require("../errors.js");
/** Buffer layout for de/serializing a multisig */
exports.MultisigLayout = (0, buffer_layout_1.struct)([
    (0, buffer_layout_1.u8)('m'),
    (0, buffer_layout_1.u8)('n'),
    (0, buffer_layout_utils_1.bool)('isInitialized'),
    (0, buffer_layout_utils_1.publicKey)('signer1'),
    (0, buffer_layout_utils_1.publicKey)('signer2'),
    (0, buffer_layout_utils_1.publicKey)('signer3'),
    (0, buffer_layout_utils_1.publicKey)('signer4'),
    (0, buffer_layout_utils_1.publicKey)('signer5'),
    (0, buffer_layout_utils_1.publicKey)('signer6'),
    (0, buffer_layout_utils_1.publicKey)('signer7'),
    (0, buffer_layout_utils_1.publicKey)('signer8'),
    (0, buffer_layout_utils_1.publicKey)('signer9'),
    (0, buffer_layout_utils_1.publicKey)('signer10'),
    (0, buffer_layout_utils_1.publicKey)('signer11'),
]);
/** Byte length of a multisig */
exports.MULTISIG_SIZE = exports.MultisigLayout.span;
/**
 * Retrieve information about a multisig
 *
 * @param connection Connection to use
 * @param address    Multisig account
 * @param commitment Desired level of commitment for querying the state
 * @param programId  SPL Token program account
 *
 * @return Multisig information
 */
function getMultisig(connection, address, commitment, programId = constants_js_1.TOKEN_PROGRAM_ID) {
    return __awaiter(this, void 0, void 0, function* () {
        const info = yield connection.getAccountInfo(address, commitment);
        return unpackMultisig(address, info, programId);
    });
}
exports.getMultisig = getMultisig;
/**
 * Unpack a multisig
 *
 * @param address   Multisig account
 * @param info      Multisig account data
 * @param programId SPL Token program account
 *
 * @return Unpacked multisig
 */
function unpackMultisig(address, info, programId = constants_js_1.TOKEN_PROGRAM_ID) {
    if (!info)
        throw new errors_js_1.TokenAccountNotFoundError();
    if (!info.owner.equals(programId))
        throw new errors_js_1.TokenInvalidAccountOwnerError();
    if (info.data.length != exports.MULTISIG_SIZE)
        throw new errors_js_1.TokenInvalidAccountSizeError();
    const multisig = exports.MultisigLayout.decode(info.data);
    return Object.assign({ address }, multisig);
}
exports.unpackMultisig = unpackMultisig;
/** Get the minimum lamport balance for a multisig to be rent exempt
 *
 * @param connection Connection to use
 * @param commitment Desired level of commitment for querying the state
 *
 * @return Amount of lamports required
 */
function getMinimumBalanceForRentExemptMultisig(connection, commitment) {
    return __awaiter(this, void 0, void 0, function* () {
        return yield connection.getMinimumBalanceForRentExemption(exports.MULTISIG_SIZE, commitment);
    });
}
exports.getMinimumBalanceForRentExemptMultisig = getMinimumBalanceForRentExemptMultisig;
//# sourceMappingURL=multisig.js.map