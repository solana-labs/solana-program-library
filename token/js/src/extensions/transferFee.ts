import { Layout, struct } from '@solana/buffer-layout';
import { Mint } from '../state/mint';
import { ExtensionType, getExtensionData } from './extensionType';


/** TransferFeeAmount as stored by the program */
export interface TransferFeeAmount {
    feeAmount: Number;
}
export declare const feeAmount: (property?: number | undefined) => Layout<Number>;

/** Buffer layout for de/serializing a mint */
export const TransferFeeAmountLayout = struct<TransferFeeAmount>([feeAmount(0)]);

export const TRANSFER_FEE_AMOUNT_SIZE = TransferFeeAmountLayout.span;

export function getTransferFeeAmount(mint: Mint): TransferFeeAmount | null {
    const extensionData = getExtensionData(ExtensionType.TransferFeeAmount, mint.tlvData);
    if (extensionData !== null) {
        return TransferFeeAmountLayout.decode(extensionData);
    } else {
        return null;
    }
}
