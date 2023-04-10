import { checkIfAccountExists } from "@/utils/retrieveData";
import {
  createAssociatedTokenAccountInstruction,
  createCloseAccountInstruction,
  createSyncNativeInstruction,
  getAssociatedTokenAddress,
  NATIVE_MINT,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import {
  Connection,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";

export async function createAtaIfNeeded(
  publicKey: PublicKey,
  payer: PublicKey,
  mint: PublicKey,
  connection: Connection
): Promise<TransactionInstruction | null> {
  const associatedTokenAccount = await getAssociatedTokenAddress(
    mint,
    publicKey
  );

  console.log("creating ata", associatedTokenAccount.toString());
  if (!(await checkIfAccountExists(associatedTokenAccount, connection))) {
    console.log("ata doesn't exist");
    return createAssociatedTokenAccountInstruction(
      payer,
      associatedTokenAccount,
      publicKey,
      mint
    );
  }

  return null;
}

export async function wrapSolIfNeeded(
  publicKey: PublicKey,
  payer: PublicKey,
  connection: Connection,
  payAmount: number
): Promise<TransactionInstruction[] | null> {
  console.log("in wrap sol if needed");
  let preInstructions: TransactionInstruction[] = [];

  const associatedTokenAccount = await getAssociatedTokenAddress(
    NATIVE_MINT,
    publicKey
  );

  const balance =
    (await connection.getBalance(associatedTokenAccount)) / LAMPORTS_PER_SOL;

  if (balance < payAmount) {
    console.log("balance insufficient");

    preInstructions.push(
      SystemProgram.transfer({
        fromPubkey: publicKey,
        toPubkey: associatedTokenAccount,
        lamports: Math.floor((payAmount - balance) * LAMPORTS_PER_SOL * 3),
      })
    );
    preInstructions.push(
      createSyncNativeInstruction(associatedTokenAccount, TOKEN_PROGRAM_ID)
    );
  }

  return preInstructions.length > 0 ? preInstructions : null;
}

export async function unwrapSolIfNeeded(
  publicKey: PublicKey,
  payer: PublicKey,
  connection: Connection
): Promise<TransactionInstruction[] | null> {
  console.log("in unwrap sol if needed");
  let preInstructions: TransactionInstruction[] = [];

  const associatedTokenAccount = await getAssociatedTokenAddress(
    NATIVE_MINT,
    publicKey
  );

  // const balance =
  //   (await connection.getBalance(associatedTokenAccount)) / LAMPORTS_PER_SOL;
  const balance = 1;

  if (balance > 0) {
    preInstructions.push(
      createCloseAccountInstruction(
        associatedTokenAccount,
        publicKey,
        publicKey
      )
    );
  }

  console.log("unwrap sol ix", preInstructions);

  return preInstructions.length > 0 ? preInstructions : null;
}
