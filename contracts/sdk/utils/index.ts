import { PublicKey, TransactionInstruction, Transaction, Signer } from "@solana/web3.js";
import * as borsh from "borsh";
import { bignum } from "@metaplex-foundation/beet";
import { BN, Provider } from "@project-serum/anchor";

export const CANDY_WRAPPER_PROGRAM_ID = new PublicKey("WRAPYChf58WFCnyjXKJHtrPgzKXgHp6MD9aVDqJBbGh");

/// Wait for a transaction of a certain id to confirm and optionally log its messages
export async function logTx(provider: Provider, txId: string, verbose: boolean = true) {
  await provider.connection.confirmTransaction(txId, "confirmed");
  if (verbose) {
    console.log(
      (await provider.connection.getConfirmedTransaction(txId, "confirmed")).meta
        .logMessages
    );
  }
};

/// Execute a series of instructions in a txn
export async function execute(
  provider: Provider,
  instructions: TransactionInstruction[],
  signers: Signer[],
  skipPreflight: boolean = false,
  verbose: boolean = false,
): Promise<String> {
  let tx = new Transaction();
  instructions.map((ix) => { tx = tx.add(ix) });
  const txid = await provider.send(tx, signers, {
    commitment: "confirmed",
    skipPreflight,
  });
  await logTx(provider, txid, verbose);
  return txid;
}

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
