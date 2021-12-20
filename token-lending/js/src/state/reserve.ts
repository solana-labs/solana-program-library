import { AccountInfo, PublicKey } from '@solana/web3.js';
import BigNumber from 'bignumber.js';
import { blob, struct, u8 } from 'buffer-layout';
import { decimal, Parser, publicKey, u64 } from '../util';
import { LastUpdate, LastUpdateLayout } from './lastUpdate';

export interface Reserve {
    version: number;
    lastUpdate: LastUpdate;
    lendingMarket: PublicKey;
    liquidity: ReserveLiquidity;
    collateral: ReserveCollateral;
    config: ReserveConfig;
}

export interface ReserveLiquidity {
    mintPubkey: PublicKey;
    mintDecimals: number;
    supplyPubkey: PublicKey;
    feeReceiver: PublicKey;
    oraclePubkey: PublicKey;
    availableAmount: bigint;
    borrowedAmountWads: BigNumber;
    cumulativeBorrowRateWads: BigNumber;
    marketPrice: BigNumber;
}

export interface ReserveCollateral {
    mintPubkey: PublicKey;
    mintTotalSupply: bigint;
    supplyPubkey: PublicKey;
}

export interface ReserveConfig {
    optimalUtilizationRate: number;
    loanToValueRatio: number;
    liquidationBonus: number;
    liquidationThreshold: number;
    minBorrowRate: number;
    optimalBorrowRate: number;
    maxBorrowRate: number;
    fees: ReserveFees;
}

export interface ReserveFees {
    borrowFeeWad: bigint;
    flashLoanFeeWad: bigint;
    hostFeePercentage: number;
}

/** @internal */
export const ReserveLiquidityLayout = struct<ReserveLiquidity>(
    [
        publicKey('mintPubkey'),
        u8('mintDecimals'),
        publicKey('supplyPubkey'),
        publicKey('feeReceiver'),
        publicKey('oraclePubkey'),
        u64('availableAmount'),
        decimal('borrowedAmountWads'),
        decimal('cumulativeBorrowRateWads'),
        decimal('marketPrice'),
    ],
    'liquidity'
);

/** @internal */
export const ReserveCollateralLayout = struct<ReserveCollateral>(
    [publicKey('mintPubkey'), u64('mintTotalSupply'), publicKey('supplyPubkey')],
    'collateral'
);

/** @internal */
export const ReserveFeesLayout = struct<ReserveFees>(
    [u64('borrowFeeWad'), u64('flashLoanFeeWad'), u8('hostFeePercentage')],
    'fees'
);

/** @internal */
export const ReserveConfigLayout = struct<ReserveConfig>(
    [
        u8('optimalUtilizationRate'),
        u8('loanToValueRatio'),
        u8('liquidationBonus'),
        u8('liquidationThreshold'),
        u8('minBorrowRate'),
        u8('optimalBorrowRate'),
        u8('maxBorrowRate'),
        ReserveFeesLayout,
    ],
    'config'
);

/** @internal */
export const ReserveLayout = struct<Reserve>([
    u8('version'),
    LastUpdateLayout,
    publicKey('lendingMarket'),
    ReserveLiquidityLayout,
    ReserveCollateralLayout,
    ReserveConfigLayout,
    blob(248, 'padding'),
]);

export const RESERVE_SIZE = ReserveLayout.span;

export const isReserve = (info: AccountInfo<Buffer>): boolean => {
    return info.data.length === RESERVE_SIZE;
};

export const parseReserve: Parser<Reserve> = (pubkey: PublicKey, info: AccountInfo<Buffer>) => {
    if (!isReserve(info)) return;

    const buffer = Buffer.from(info.data);
    const reserve = ReserveLayout.decode(buffer);

    if (!reserve.version) return;

    return {
        pubkey,
        info,
        data: reserve,
    };
};
