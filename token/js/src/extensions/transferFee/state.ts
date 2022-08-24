import type { Layout } from '@solana/buffer-layout';
import { struct, u16 } from '@solana/buffer-layout';
import { publicKey, u64 } from '@solana/buffer-layout-utils';
import type { PublicKey } from '@solana/web3.js';
import type { Account } from '../../state/account.js';
import type { Mint } from '../../state/mint.js';
import { ExtensionType, getExtensionData } from '../extensionType.js';

export const MAX_FEE_BASIS_POINTS = 10_000;
export const ONE_IN_BASIS_POINTS: bigint = MAX_FEE_BASIS_POINTS as unknown as bigint;

/** TransferFeeConfig as stored by the program */
export interface TransferFee {
    /** First epoch where the transfer fee takes effect */
    epoch: bigint;
    /** Maximum fee assessed on transfers, expressed as an amount of tokens */
    maximumFee: bigint;
    /**
     * Amount of transfer collected as fees, expressed as basis points of the
     * transfer amount, ie. increments of 0.01%
     */
    transferFeeBasisPoints: number;
}

/** Transfer fee extension data for mints. */
export interface TransferFeeConfig {
    /** Optional authority to set the fee */
    transferFeeConfigAuthority: PublicKey;
    /** Withdraw from mint instructions must be signed by this key */
    withdrawWithheldAuthority: PublicKey;
    /** Withheld transfer fee tokens that have been moved to the mint for withdrawal */
    withheldAmount: bigint;
    /** Older transfer fee, used if the current epoch < newerTransferFee.epoch */
    olderTransferFee: TransferFee;
    /** Newer transfer fee, used if the current epoch >= newerTransferFee.epoch */
    newerTransferFee: TransferFee;
}

/** Buffer layout for de/serializing a transfer fee */
export function transferFeeLayout(property?: string): Layout<TransferFee> {
    return struct<TransferFee>([u64('epoch'), u64('maximumFee'), u16('transferFeeBasisPoints')], property);
}

/** Buffer layout for de/serializing a transfer fee config extension */
export const TransferFeeConfigLayout = struct<TransferFeeConfig>([
    publicKey('transferFeeConfigAuthority'),
    publicKey('withdrawWithheldAuthority'),
    u64('withheldAmount'),
    transferFeeLayout('olderTransferFee'),
    transferFeeLayout('newerTransferFee'),
]);

export const TRANSFER_FEE_CONFIG_SIZE = TransferFeeConfigLayout.span;

/** Transfer fee amount data for accounts. */
export interface TransferFeeAmount {
    /** Withheld transfer fee tokens that can be claimed by the fee authority */
    withheldAmount: bigint;
}
/** Buffer layout for de/serializing */
export const TransferFeeAmountLayout = struct<TransferFeeAmount>([u64('withheldAmount')]);
export const TRANSFER_FEE_AMOUNT_SIZE = TransferFeeAmountLayout.span;

export function getTransferFeeConfig(mint: Mint): TransferFeeConfig | null {
    const extensionData = getExtensionData(ExtensionType.TransferFeeConfig, mint.tlvData);
    if (extensionData !== null) {
        return TransferFeeConfigLayout.decode(extensionData);
    } else {
        return null;
    }
}

export function getTransferFeeAmount(account: Account): TransferFeeAmount | null {
    const extensionData = getExtensionData(ExtensionType.TransferFeeAmount, account.tlvData);
    if (extensionData !== null) {
        return TransferFeeAmountLayout.decode(extensionData);
    } else {
        return null;
    }
}
