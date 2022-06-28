import { Provider } from "@project-serum/anchor";
import { TransactionInstruction, Transaction, Signer } from "@solana/web3.js";

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
  skipPreflight: boolean = false
): Promise<String> {
  let tx = new Transaction();
  instructions.map((ix) => { tx = tx.add(ix) });
  const txid = await provider.send(tx, signers, {
    commitment: "confirmed",
    skipPreflight,
  });
  await logTx(provider, txid, false);
  return txid;
}

/// Convert a 32 bit number to a buffer of bytes
export function num32ToBuffer(num: number) {
  const isU32 = (num >= 0 && num < Math.pow(2,32));
  const isI32 = (num >= -1*Math.pow(2, 31) && num < Math.pow(2,31))
  if (!isU32 || !isI32) {
    throw new Error("Attempted to convert non 32 bit integer to byte array")
  }
  var byte1 = 0xff & num;
  var byte2 = 0xff & (num >> 8);
  var byte3 = 0xff & (num >> 16);
  var byte4 = 0xff & (num >> 24);
  return Buffer.from([byte1, byte2, byte3, byte4])
}

/// Check if two Array types contain the same values in order
export function arrayEquals(a, b) {
  return Array.isArray(a) &&
      Array.isArray(b) &&
      a.length === b.length &&
      a.every((val, index) => val === b[index]);
}

/// Convert Buffer to Uint8Array
export function bufferToArray(buffer: Buffer): number[] {
  const nums = [];
  for (let i = 0; i < buffer.length; i++) {
    nums.push(buffer.at(i));
  }
  return nums;
}
