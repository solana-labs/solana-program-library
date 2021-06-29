import { AccountInfo, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { blob, seq, struct, u8 } from 'buffer-layout';
import { publicKey, u64, u128 } from '../util';
import { LastUpdate, LastUpdateLayout } from './lastUpdate';

export interface Obligation {
    version: number;
    lastUpdate: LastUpdate;
    lendingMarket: PublicKey;
    owner: PublicKey;
    deposits: ObligationCollateral[];
    borrows: ObligationLiquidity[];
    depositedValue: BN; // decimals
    borrowedValue: BN; // decimals
    allowedBorrowValue: BN; // decimals
    unhealthyBorrowValue: BN; // decimals
}

export interface ObligationCollateral {
    depositReserve: PublicKey;
    depositedAmount: BN;
    marketValue: BN; // decimals
}

export interface ObligationLiquidity {
    borrowReserve: PublicKey;
    cumulativeBorrowRateWads: BN; // decimals
    borrowedAmountWads: BN; // decimals
    marketValue: BN; // decimals
}

export interface ProtoObligation {
    version: number;
    lastUpdate: LastUpdate;
    lendingMarket: PublicKey;
    owner: PublicKey;
    depositedValue: BN; // decimals
    borrowedValue: BN; // decimals
    allowedBorrowValue: BN; // decimals
    unhealthyBorrowValue: BN; // decimals
    depositsLen: number;
    borrowsLen: number;
    dataFlat: Buffer;
}

export const ObligationCollateralLayout = struct<ObligationCollateral>([
    publicKey('depositReserve'),
    u64('depositedAmount'),
    u128('marketValue'),
]);

export const ObligationLiquidityLayout = struct<ObligationLiquidity>([
    publicKey('borrowReserve'),
    u128('cumulativeBorrowRateWads'),
    u128('borrowedAmountWads'),
    u128('marketValue'),
]);

export const ObligationLayout = struct<ProtoObligation>([
    u8('version'),

    LastUpdateLayout,

    publicKey('lendingMarket'),
    publicKey('owner'),
    u128('depositedValue'),
    u128('borrowedValue'),
    u128('allowedBorrowValue'),
    u128('unhealthyBorrowValue'),

    u8('depositsLen'),
    u8('borrowsLen'),
    blob(776, 'dataFlat'),
]);

export const isObligation = (info: AccountInfo<Buffer>) => {
    return info.data.length === ObligationLayout.span;
};

export const ObligationParser = (pubkey: PublicKey, info: AccountInfo<Buffer>) => {
    if (!isObligation(info)) return;

    const buffer = Buffer.from(info.data);
    const {
        version,
        lastUpdate,
        lendingMarket,
        owner,
        depositedValue,
        borrowedValue,
        allowedBorrowValue,
        unhealthyBorrowValue,
        depositsLen,
        borrowsLen,
        dataFlat,
    } = ObligationLayout.decode(buffer);

    if (lastUpdate.slot.isZero()) {
        return;
    }

    const depositsSpan = depositsLen * ObligationCollateralLayout.span;
    const borrowsSpan = borrowsLen * ObligationLiquidityLayout.span;

    const depositsBuffer = dataFlat.slice(0, depositsSpan);
    const deposits = seq(ObligationCollateralLayout, depositsLen).decode(depositsBuffer);

    const borrowsBuffer = dataFlat.slice(depositsSpan, depositsSpan + borrowsSpan);
    const borrows = seq(ObligationLiquidityLayout, borrowsLen).decode(borrowsBuffer);

    const obligation = {
        version,
        lastUpdate,
        lendingMarket,
        owner,
        depositedValue,
        borrowedValue,
        allowedBorrowValue,
        unhealthyBorrowValue,
        deposits,
        borrows,
    } as Obligation;

    return {
        pubkey,
        account: info,
        info: obligation,
    };
};
