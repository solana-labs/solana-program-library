import type { AccountMeta } from '@solana/web3.js';
import { TokenTransferHookInvalidSeed } from '../../errors.js';

interface Seed {
    length: number;
    data: Buffer;
}

function unpackSeedLiteral(seeds: Uint8Array): Seed {
    if (seeds.length < 1) {
        throw new TokenTransferHookInvalidSeed();
    }
    const [length, ...rest] = seeds;
    if (rest.length < length) {
        throw new TokenTransferHookInvalidSeed();
    }
    return {
        data: Buffer.from(rest.slice(0, length)),
        length: length + 2,
    };
}

function unpackSeedInstructionArg(seeds: Uint8Array, instructionData: Buffer): Seed {
    if (seeds.length < 2) {
        throw new TokenTransferHookInvalidSeed();
    }
    const [index, length] = seeds;
    if (instructionData.length < length) {
        throw new TokenTransferHookInvalidSeed();
    }
    return {
        data: instructionData.subarray(index, index + length),
        length: 3,
    };
}

function unpackSeedAccountKey(seeds: Uint8Array, previousMetas: AccountMeta[]): Seed {
    if (seeds.length < 1) {
        throw new TokenTransferHookInvalidSeed();
    }
    const [index] = seeds;
    if (previousMetas.length <= index) {
        throw new TokenTransferHookInvalidSeed();
    }
    return {
        data: previousMetas[index].pubkey.toBuffer(),
        length: 2,
    };
}

function unpackFirstSeed(seeds: Uint8Array, previousMetas: AccountMeta[], instructionData: Buffer): Seed | null {
    const [discriminator, ...rest] = seeds;
    const remaining = new Uint8Array(rest);
    switch (discriminator) {
        case 0:
            return null;
        case 1:
            return unpackSeedLiteral(remaining);
        case 2:
            return unpackSeedInstructionArg(remaining, instructionData);
        case 3:
            return unpackSeedAccountKey(remaining, previousMetas);
        default:
            throw new TokenTransferHookInvalidSeed();
    }
}

export function unpackSeeds(seeds: Uint8Array, previousMetas: AccountMeta[], instructionData: Buffer): Buffer[] {
    const unpackedSeeds: Buffer[] = [];
    let i = 0;
    while (i < 32) {
        const seed = unpackFirstSeed(seeds.slice(i), previousMetas, instructionData);
        if (seed == null) {
            break;
        }
        unpackedSeeds.push(seed.data);
        i += seed.length;
    }
    return unpackedSeeds;
}
