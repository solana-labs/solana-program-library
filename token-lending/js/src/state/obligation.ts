import { AccountInfo, PublicKey } from '@solana/web3.js';
import BigNumber from 'bignumber.js';
import { blob, seq, struct, u8 } from 'buffer-layout';
import { decimal, Parser, publicKey, u64 } from '../util';
import { LastUpdate, LastUpdateLayout } from './lastUpdate';

export interface Obligation {
    version: number;
    lastUpdate: LastUpdate;
    lendingMarket: PublicKey;
    owner: PublicKey;
    deposits: ObligationCollateral[];
    borrows: ObligationLiquidity[];
    depositedValue: BigNumber;
    borrowedValue: BigNumber;
    allowedBorrowValue: BigNumber;
    unhealthyBorrowValue: BigNumber;
}

export interface ObligationCollateral {
    depositReserve: PublicKey;
    depositedAmount: bigint;
    marketValue: BigNumber;
}

export interface ObligationLiquidity {
    borrowReserve: PublicKey;
    cumulativeBorrowRateWads: BigNumber;
    borrowedAmountWads: BigNumber;
    marketValue: BigNumber;
}

/** @internal */
export interface ObligationDataFlat {
    version: number;
    lastUpdate: LastUpdate;
    lendingMarket: PublicKey;
    owner: PublicKey;
    depositedValue: BigNumber;
    borrowedValue: BigNumber;
    allowedBorrowValue: BigNumber;
    unhealthyBorrowValue: BigNumber;
    depositsLen: number;
    borrowsLen: number;
    dataFlat: Buffer;
}

/** @internal */
export const ObligationCollateralLayout = struct<ObligationCollateral>(
    [publicKey('depositReserve'), u64('depositedAmount'), decimal('marketValue')],
    'collateral'
);

/** @internal */
export const ObligationLiquidityLayout = struct<ObligationLiquidity>(
    [
        publicKey('borrowReserve'),
        decimal('cumulativeBorrowRateWads'),
        decimal('borrowedAmountWads'),
        decimal('marketValue'),
    ],
    'liquidity'
);

/** @internal */
export const ObligationLayout = struct<ObligationDataFlat>(
    [
        u8('version'),
        LastUpdateLayout,
        publicKey('lendingMarket'),
        publicKey('owner'),
        decimal('depositedValue'),
        decimal('borrowedValue'),
        decimal('allowedBorrowValue'),
        decimal('unhealthyBorrowValue'),
        u8('depositsLen'),
        u8('borrowsLen'),
        blob(ObligationCollateralLayout.span + 9 * ObligationLiquidityLayout.span, 'dataFlat'),
    ],
    'obligation'
);

export const OBLIGATION_SIZE = ObligationLayout.span;

export const isObligation = (info: AccountInfo<Buffer>): boolean => {
    return info.data.length === OBLIGATION_SIZE;
};

export const parseObligation: Parser<Obligation> = (pubkey: PublicKey, info: AccountInfo<Buffer>) => {
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

    if (!version) return;

    const depositsSpan = depositsLen * ObligationCollateralLayout.span;
    const borrowsSpan = borrowsLen * ObligationLiquidityLayout.span;

    const depositsBuffer = dataFlat.slice(0, depositsSpan);
    const deposits = seq(ObligationCollateralLayout, depositsLen).decode(depositsBuffer);

    const borrowsBuffer = dataFlat.slice(depositsSpan, depositsSpan + borrowsSpan);
    const borrows = seq(ObligationLiquidityLayout, borrowsLen).decode(borrowsBuffer);

    const obligation: Obligation = {
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
    };

    return {
        pubkey,
        info,
        data: obligation,
    };
};
