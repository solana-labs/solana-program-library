"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.getTransferFeeAmount = exports.getTransferFeeConfig = exports.TRANSFER_FEE_AMOUNT_SIZE = exports.TransferFeeAmountLayout = exports.TRANSFER_FEE_CONFIG_SIZE = exports.TransferFeeConfigLayout = exports.transferFeeLayout = exports.ONE_IN_BASIS_POINTS = exports.MAX_FEE_BASIS_POINTS = void 0;
const buffer_layout_1 = require("@solana/buffer-layout");
const buffer_layout_utils_1 = require("@solana/buffer-layout-utils");
const extensionType_js_1 = require("../extensionType.js");
exports.MAX_FEE_BASIS_POINTS = 10000;
exports.ONE_IN_BASIS_POINTS = exports.MAX_FEE_BASIS_POINTS;
/** Buffer layout for de/serializing a transfer fee */
function transferFeeLayout(property) {
    return (0, buffer_layout_1.struct)([(0, buffer_layout_utils_1.u64)('epoch'), (0, buffer_layout_utils_1.u64)('maximumFee'), (0, buffer_layout_1.u16)('transferFeeBasisPoints')], property);
}
exports.transferFeeLayout = transferFeeLayout;
/** Buffer layout for de/serializing a transfer fee config extension */
exports.TransferFeeConfigLayout = (0, buffer_layout_1.struct)([
    (0, buffer_layout_utils_1.publicKey)('transferFeeConfigAuthority'),
    (0, buffer_layout_utils_1.publicKey)('withdrawWithheldAuthority'),
    (0, buffer_layout_utils_1.u64)('withheldAmount'),
    transferFeeLayout('olderTransferFee'),
    transferFeeLayout('newerTransferFee'),
]);
exports.TRANSFER_FEE_CONFIG_SIZE = exports.TransferFeeConfigLayout.span;
/** Buffer layout for de/serializing */
exports.TransferFeeAmountLayout = (0, buffer_layout_1.struct)([(0, buffer_layout_utils_1.u64)('withheldAmount')]);
exports.TRANSFER_FEE_AMOUNT_SIZE = exports.TransferFeeAmountLayout.span;
function getTransferFeeConfig(mint) {
    const extensionData = (0, extensionType_js_1.getExtensionData)(extensionType_js_1.ExtensionType.TransferFeeConfig, mint.tlvData);
    if (extensionData !== null) {
        return exports.TransferFeeConfigLayout.decode(extensionData);
    }
    else {
        return null;
    }
}
exports.getTransferFeeConfig = getTransferFeeConfig;
function getTransferFeeAmount(account) {
    const extensionData = (0, extensionType_js_1.getExtensionData)(extensionType_js_1.ExtensionType.TransferFeeAmount, account.tlvData);
    if (extensionData !== null) {
        return exports.TransferFeeAmountLayout.decode(extensionData);
    }
    else {
        return null;
    }
}
exports.getTransferFeeAmount = getTransferFeeAmount;
//# sourceMappingURL=state.js.map