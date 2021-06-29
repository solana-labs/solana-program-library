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

export const LendingMarketLayout = struct<LendingMarket>([
    u8('version'),
    u8('bumpSeed'),
    publicKey('owner'),
    blob(32, 'quoteCurrency'),
    publicKey('tokenProgramId'),
    publicKey('oracleProgramId'),
    blob(128, 'padding'),
]);

export const isLendingMarket = (info: AccountInfo<Buffer>): boolean => {
    return info.data.length === LendingMarketLayout.span;
};

export const LendingMarketParser: Parser<LendingMarket> = (pubkey: PublicKey, info: AccountInfo<Buffer>) => {
    if (!isLendingMarket(info)) return;

    const buffer = Buffer.from(info.data);
    const lendingMarket = LendingMarketLayout.decode(buffer);

    return {
        pubkey,
        account: info,
        info: lendingMarket,
    };
};
