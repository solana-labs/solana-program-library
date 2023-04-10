import { CustodyAccount } from "@/lib/CustodyAccount";
import { PoolAccount } from "@/lib/PoolAccount";
import { PositionAccount } from "@/lib/PositionAccount";
import { TokenE } from "@/lib/Token";
import {
  getPerpetualProgramAndProvider,
  PERPETUALS_ADDRESS,
  TRANSFER_AUTHORITY,
} from "@/utils/constants";
import {
  automaticSendTransaction,
  manualSendTransaction,
} from "@/utils/TransactionHandlers";
import {
  createAtaIfNeeded,
  unwrapSolIfNeeded,
} from "@/utils/transactionHelpers";
import { BN } from "@project-serum/anchor";
import { getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { WalletContextState } from "@solana/wallet-adapter-react";
import { Connection, TransactionInstruction } from "@solana/web3.js";

export async function closePosition(
  walletContextState: WalletContextState,
  connection: Connection,
  pool: PoolAccount,
  position: PositionAccount,
  custody: CustodyAccount,
  price: BN
) {
  let { perpetual_program } = await getPerpetualProgramAndProvider(
    walletContextState
  );
  let publicKey = walletContextState.publicKey!;

  // TODO: need to take slippage as param , this is now for testing
  const adjustedPrice =
    position.side.toString() == "Long"
      ? price.mul(new BN(50)).div(new BN(100))
      : price.mul(new BN(150)).div(new BN(100));

  let userCustodyTokenAccount = await getAssociatedTokenAddress(
    custody.mint,
    publicKey
  );

  let preInstructions: TransactionInstruction[] = [];

  let ataIx = await createAtaIfNeeded(
    publicKey,
    publicKey,
    custody.mint,
    connection
  );

  if (ataIx) preInstructions.push(ataIx);

  let postInstructions: TransactionInstruction[] = [];
  let unwrapTx = await unwrapSolIfNeeded(publicKey, publicKey, connection);
  if (unwrapTx) postInstructions.push(...unwrapTx);

  let methodBuilder = await perpetual_program.methods
    .closePosition({
      price: adjustedPrice,
    })
    .accounts({
      owner: publicKey,
      receivingAccount: userCustodyTokenAccount,
      transferAuthority: TRANSFER_AUTHORITY,
      perpetuals: PERPETUALS_ADDRESS,
      pool: pool.address,
      position: position.address,
      custody: custody.address,
      custodyOracleAccount: custody.oracle.oracleAccount,
      custodyTokenAccount: custody.tokenAccount,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .preInstructions(preInstructions);

  if (position.token == TokenE.SOL)
    methodBuilder = methodBuilder.postInstructions(postInstructions);

  try {
    // await automaticSendTransaction(
    //   methodBuilder,
    //   perpetual_program.provider.connection
    // );
    let tx = await methodBuilder.transaction();
    await manualSendTransaction(
      tx,
      publicKey,
      connection,
      walletContextState.signTransaction
    );
  } catch (err) {
    console.log(err);
    throw err;
  }
}
