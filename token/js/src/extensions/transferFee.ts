import { struct, u16 } from '@solana/buffer-layout';
import { publicKey, u64 } from '@solana/buffer-layout-utils';
import { PublicKey } from '@solana/web3.js';
import { Account } from '../state';
import { Mint } from '../state/mint';
import { ExtensionType, getExtensionData } from './extensionType';

export const MAX_FEE_BASIS_POINTS = 10_000;
export const ONE_IN_BASIS_POINTS: BigInt = MAX_FEE_BASIS_POINTS as unknown as BigInt;
/** TransferFeeConfig as stored by the program */
export interface TransferFee {
    /** First epoch where the transfer fee takes effect*/
    epoch: BigInt;
    /**Maximum fee assessed on transfers, expressed as an amount of tokens */
    maximum_fee: BigInt;
    /** Amount of transfer collected as fees, expressed as basis points of the */
    /** transfer amount, ie. increments of 0.01% */
    transfer_fee_basis_points: number;
}

/// Transfer fee extension data for mints.
export interface TransferFeeConfig {
    /// Optional authority to set the fee
    transfer_fee_config_authority: PublicKey;
    /// Withdraw from mint instructions must be signed by this key
    withdraw_withheld_authority: PublicKey;
    /// Withheld transfer fee tokens that have been moved to the mint for withdrawal
    withheld_amount: BigInt;
    /// Older transfer fee, used if the current epoch < new_transfer_fee.epoch
    older_transfer_fee: TransferFee;
    /// Newer transfer fee, used if the current epoch >= new_transfer_fee.epoch
    newer_transfer_fee: TransferFee;
}

/** Buffer layout for de/serializing a mint */
export const TransferFeeLayout = struct<TransferFee>([
    u64('epoch'),
    u64('maximum_fee'),
    u16('transfer_fee_basis_point'),
]);

export const TRANSFER_FEE_AMOUNT_SIZE = TransferFeeLayout.span;

/** Buffer layout for de/serializing a mint */
export const TransferFeeConfigLayout = struct<TransferFeeConfig>([
    publicKey('transfer_fee_config_authority'),
    publicKey('withdraw_withheld_authority'),
    u64('withheld_amount'),
    TransferFeeLayout,
    TransferFeeLayout,
]);

export const TRANSFER_FEE_CONFIG_SIZE = TransferFeeConfigLayout.span;

export function getTransferFeeConfig(mint: Mint): TransferFeeConfig | null {
    const extensionData = getExtensionData(ExtensionType.TransferFeeConfig, mint.tlvData);
    if (extensionData !== null) {
        return TransferFeeConfigLayout.decode(extensionData);
    } else {
        return null;
    }
}

export function getTransferFee(account: Account): TransferFee | null {
    const extensionData = getExtensionData(ExtensionType.TransferFeeConfig, account.tlvData);
    if (extensionData !== null) {
        return TransferFeeLayout.decode(extensionData);
    } else {
        return null;
    }
}
