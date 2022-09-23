import { keccak_256 } from "js-sha3";

/**
 * Replicates on-chain hash function to hash together buffers
 */
export function hash(left: Buffer, right: Buffer): Buffer {
    return Buffer.from(keccak_256.digest(Buffer.concat([left, right])));
}