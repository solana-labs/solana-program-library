import { struct, u16 } from '@solana/buffer-layout';
import { publicKey, u64 } from '@solana/buffer-layout-utils';
import { ExtensionType, getExtensionData } from '../extensionType.js';
export const MAX_FEE_BASIS_POINTS = 10000;
export const ONE_IN_BASIS_POINTS = MAX_FEE_BASIS_POINTS;
/** Buffer layout for de/serializing a transfer fee */
export function transferFeeLayout(property) {
    return struct([u64('epoch'), u64('maximumFee'), u16('transferFeeBasisPoints')], property);
}
/** Buffer layout for de/serializing a transfer fee config extension */
export const TransferFeeConfigLayout = struct([
    publicKey('transferFeeConfigAuthority'),
    publicKey('withdrawWithheldAuthority'),
    u64('withheldAmount'),
    transferFeeLayout('olderTransferFee'),
    transferFeeLayout('newerTransferFee'),
]);
export const TRANSFER_FEE_CONFIG_SIZE = TransferFeeConfigLayout.span;
/** Buffer layout for de/serializing */
export const TransferFeeAmountLayout = struct([u64('withheldAmount')]);
export const TRANSFER_FEE_AMOUNT_SIZE = TransferFeeAmountLayout.span;
export function getTransferFeeConfig(mint) {
    const extensionData = getExtensionData(ExtensionType.TransferFeeConfig, mint.tlvData);
    if (extensionData !== null) {
        return TransferFeeConfigLayout.decode(extensionData);
    }
    else {
        return null;
    }
}
export function getTransferFeeAmount(account) {
    const extensionData = getExtensionData(ExtensionType.TransferFeeAmount, account.tlvData);
    if (extensionData !== null) {
        return TransferFeeAmountLayout.decode(extensionData);
    }
    else {
        return null;
    }
}
//# sourceMappingURL=state.js.map