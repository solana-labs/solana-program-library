import {
  PublicKey,
} from "@solana/web3.js";
import * as borsh from "borsh";
import { bignum } from "@metaplex-foundation/beet";
import * as BN from 'bn.js';

export const LOG_WRAPPER_PROGRAM_ID = new PublicKey("WRAPYChf58WFCnyjXKJHtrPgzKXgHp6MD9aVDqJBbGh");

/// Read in a public key from a BinaryReader
export function readPublicKey(reader: borsh.BinaryReader): PublicKey {
  return new PublicKey(reader.readFixedArray(32));
}

/// Extract the value of a Metaplex Bignum
export function val(num: bignum): BN {
  if (BN.isBN(num)) {
    return num;
  }
  return new BN(num);
}

/// Convert a string to a byte array, stored as an array of numbers
export function strToByteArray(str: string, padTo?: number): number[] {
  let buf: Buffer = Buffer.from(
    [...str].reduce((acc: number[], c, ind) => acc.concat([str.charCodeAt(ind)]), [])
  );
  if (padTo) {
    buf = Buffer.concat([buf], padTo);
  }
  return [...buf];
}

/// Convert a string to a byte array, stored in a Uint8Array
export function strToByteUint8Array(str: string): Uint8Array {
  return Uint8Array.from(
    [...str].reduce((acc: number[], c, ind) => acc.concat([str.charCodeAt(ind)]), [])
  );
}

/// Convert a 32 bit number to a buffer of bytes
export function num32ToBuffer(num: number) {
  const isU32 = (num >= 0 && num < Math.pow(2, 32));
  if (!isU32) {
    throw new Error("Attempted to convert non 32 bit integer to byte array")
  }
  const b = Buffer.alloc(4);
  b.writeInt32LE(num);
  return b;
}

/// Convert a 16 bit number to a buffer of bytes
export function num16ToBuffer(num: number) {
  const isU16 = (num >= 0 && num < Math.pow(2, 16));
  if (!isU16) {
    throw new Error("Attempted to convert non 16 bit integer to byte array")
  }
  const b = Buffer.alloc(2);
  b.writeUInt16LE(num);
  return b;
}

/// Check if two Array types contain the same values in order
export function arrayEquals(a: any, b: any) {
  return Array.isArray(a) &&
    Array.isArray(b) &&
    a.length === b.length &&
    a.every((val, index) => val === b[index]);
}

/// Convert Buffer to Uint8Array
export function bufferToArray(buffer: Buffer): number[] {
  const nums: number[] = [];
  for (let i = 0; i < buffer.length; i++) {
    nums.push(buffer[i]);
  }
  return nums;
}

/// Remove null characters from a string. Useful for comparring byte-padded on-chain strings with off-chain values
export const trimStringPadding = (str: string): string => {
  return str.replace(/\0/g, '')
}

