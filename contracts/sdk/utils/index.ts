import { PublicKey } from "@solana/web3.js";
import * as borsh from "borsh";
import { bignum } from "@metaplex-foundation/beet";
import { BN } from "@project-serum/anchor";

export const CANDY_WRAPPER_PROGRAM_ID = new PublicKey("WRAPYChf58WFCnyjXKJHtrPgzKXgHp6MD9aVDqJBbGh");

export function readPublicKey(reader: borsh.BinaryReader): PublicKey {
  return new PublicKey(reader.readFixedArray(32));
}

export function val(num: bignum): BN {
  if (BN.isBN(num)) {
    return num;
  }
  return new BN(num);
}

export function strToByteArray(str: string, padTo?: number): number[] {
  let buf: Buffer = Buffer.from(
    [...str].reduce((acc, c, ind) => acc.concat([str.charCodeAt(ind)]), [])
  );
  if (padTo) {
    buf = Buffer.concat([buf], padTo);
  }
  return [...buf];
}

export function strToByteUint8Array(str: string): Uint8Array {
  return Uint8Array.from(
    [...str].reduce((acc, c, ind) => acc.concat([str.charCodeAt(ind)]), [])
  );
}

export async function getBubblegumAuthorityPDAKey(merkleRollPubKey: PublicKey, bubblegumProgramId: PublicKey) {
    const [bubblegumAuthorityPDAKey] = await PublicKey.findProgramAddress(
      [merkleRollPubKey.toBuffer()],
      bubblegumProgramId
    );
    return bubblegumAuthorityPDAKey;
}
