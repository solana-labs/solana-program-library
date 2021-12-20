import { AccountInfo, PublicKey } from '@solana/web3.js';
import { blob, struct, u8 } from 'buffer-layout';
import { Parser, publicKey } from '../util';

export interface LendingMarket {
    version: number;
    bumpSeed: number;
    owner: PublicKey;
    quoteCurrency: Buffer;
    tokenProgramId: PublicKey;
    oracleProgramId: PublicKey;
}

/** @internal */
export const LendingMarketLayout = struct<LendingMarket>(
    [
        u8('version'),
        u8('bumpSeed'),
        publicKey('owner'),
        blob(32, 'quoteCurrency'),
        publicKey('tokenProgramId'),
        publicKey('oracleProgramId'),
        blob(128, 'padding'),
    ],
    'lendingMarket'
);

export const LENDING_MARKET_SIZE = LendingMarketLayout.span;

export const isLendingMarket = (info: AccountInfo<Buffer>): boolean => {
    return info.data.length === LENDING_MARKET_SIZE;
};

export const parseLendingMarket: Parser<LendingMarket> = (pubkey: PublicKey, info: AccountInfo<Buffer>) => {
    if (!isLendingMarket(info)) return;

    const buffer = Buffer.from(info.data);
    const lendingMarket = LendingMarketLayout.decode(buffer);

    if (!lendingMarket.version) return;

    return {
        pubkey,
        info,
        data: lendingMarket,
    };
};
