import {
  PublicKey,
} from "@solana/web3.js";
import * as borsh from "borsh";

export const SPL_NOOP_ADDRESS = "WRAPYChf58WFCnyjXKJHtrPgzKXgHp6MD9aVDqJBbGh";
export const SPL_NOOP_PROGRAM_ID = new PublicKey("WRAPYChf58WFCnyjXKJHtrPgzKXgHp6MD9aVDqJBbGh");

/// Read in a public key from a BinaryReader
export function readPublicKey(reader: borsh.BinaryReader): PublicKey {
  return new PublicKey(reader.readFixedArray(32));
}