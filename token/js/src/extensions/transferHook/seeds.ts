import type { AccountMeta } from '@solana/web3.js';
import { TokenTransferHookInvalidSeed } from '../../errors.js';

interface Seed {
    data: Buffer;
    packedLength: number;
}

const DISCRIMINATOR_SPAN = 1;
const LITERAL_LENGTH_SPAN = 1;
const INSTRUCTION_ARG_OFFSET_SPAN = 1;
const INSTRUCTION_ARG_LENGTH_SPAN = 1;
const ACCOUNT_KEY_INDEX_SPAN = 1;

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
        packedLength: DISCRIMINATOR_SPAN + LITERAL_LENGTH_SPAN + length,
    };
}

function unpackSeedInstructionArg(seeds: Uint8Array, instructionData: Buffer): Seed {
    if (seeds.length < 2) {
        throw new TokenTransferHookInvalidSeed();
    }
    const [index, length] = seeds;
    if (instructionData.length < length + index) {
        throw new TokenTransferHookInvalidSeed();
    }
    return {
        data: instructionData.subarray(index, index + length),
        packedLength: DISCRIMINATOR_SPAN + INSTRUCTION_ARG_OFFSET_SPAN + INSTRUCTION_ARG_LENGTH_SPAN,
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
        packedLength: DISCRIMINATOR_SPAN + ACCOUNT_KEY_INDEX_SPAN,
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
        i += seed.packedLength;
    }
    return unpackedSeeds;
}
