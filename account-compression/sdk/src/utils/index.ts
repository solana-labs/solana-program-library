import {
  PublicKey,
} from "@solana/web3.js";
import * as borsh from "borsh";

export const LOG_WRAPPER_PROGRAM_ID = new PublicKey("WRAPYChf58WFCnyjXKJHtrPgzKXgHp6MD9aVDqJBbGh");

/// Read in a public key from a BinaryReader
export function readPublicKey(reader: borsh.BinaryReader): PublicKey {
  return new PublicKey(reader.readFixedArray(32));
}