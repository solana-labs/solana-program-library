import { AccountInfo, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { blob, struct, u8 } from 'buffer-layout';
import { publicKey, u64, u128 } from '../util';
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
    availableAmount: BN;
    borrowedAmountWads: BN; // decimals
    cumulativeBorrowRateWads: BN; // decimals
    marketPrice: BN; // decimals
}

export interface ReserveCollateral {
    mintPubkey: PublicKey;
    mintTotalSupply: BN;
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
    fees: {
        borrowFeeWad: BN;
        hostFeePercentage: number;
    };
}

export const ReserveLayout = struct<Reserve>([
    u8('version'),

    LastUpdateLayout,

    publicKey('lendingMarket'),

    struct(
        [
            publicKey('mintPubkey'),
            u8('mintDecimals'),
            publicKey('supplyPubkey'),
            publicKey('feeReceiver'),
            publicKey('oraclePubkey'),
            u64('availableAmount'),
            u128('borrowedAmountWads'),
            u128('cumulativeBorrowRateWads'),
            u128('marketPrice'),
        ],
        'liquidity'
    ),

    struct([publicKey('mintPubkey'), u64('mintTotalSupply'), publicKey('supplyPubkey')], 'collateral'),

    struct(
        [
            u8('optimalUtilizationRate'),
            u8('loanToValueRatio'),
            u8('liquidationBonus'),
            u8('liquidationThreshold'),
            u8('minBorrowRate'),
            u8('optimalBorrowRate'),
            u8('maxBorrowRate'),
            struct([u64('borrowFeeWad'), u64('flashLoanFeeWad'), u8('hostFeePercentage')], 'fees'),
        ],
        'config'
    ),

    blob(248, 'padding'),
]);

export const isReserve = (info: AccountInfo<Buffer>) => {
    return info.data.length === ReserveLayout.span;
};

export const ReserveParser = (pubkey: PublicKey, info: AccountInfo<Buffer>) => {
    if (!isReserve(info)) return;

    const buffer = Buffer.from(info.data);
    const reserve = ReserveLayout.decode(buffer);

    if (reserve.lastUpdate.slot.isZero()) return;

    return {
        pubkey,
        account: info,
        info: reserve,
    };
};
