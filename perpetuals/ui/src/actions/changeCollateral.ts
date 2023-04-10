import { PoolAccount } from "@/lib/PoolAccount";
import { PositionAccount } from "@/lib/PositionAccount";
import { TokenE } from "@/lib/Token";
import { Tab } from "@/lib/types";
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
  wrapSolIfNeeded,
} from "@/utils/transactionHelpers";
import { BN } from "@project-serum/anchor";
import {
  getAssociatedTokenAddress,
  NATIVE_MINT,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { WalletContextState } from "@solana/wallet-adapter-react";
import { Connection, TransactionInstruction } from "@solana/web3.js";

export async function changeCollateral(
  walletContextState: WalletContextState,
  connection: Connection,
  pool: PoolAccount,
  position: PositionAccount,
  collatNum: number,
  tab: Tab
) {
  let { perpetual_program } = await getPerpetualProgramAndProvider(
    walletContextState
  );

  let publicKey = walletContextState.publicKey!;

  let custody = pool.getCustodyAccount(position.token)!;

  let userCustodyTokenAccount = await getAssociatedTokenAddress(
    custody.mint,
    publicKey
  );

  let preInstructions: TransactionInstruction[] = [];

  let methodBuilder;
  let postInstructions: TransactionInstruction[] = [];
  let unwrapTx = await unwrapSolIfNeeded(publicKey, publicKey, connection);
  if (unwrapTx) postInstructions.push(...unwrapTx);

  if (tab == Tab.Add) {
    if (position.token == TokenE.SOL) {
      let ataIx = await createAtaIfNeeded(
        publicKey,
        publicKey,
        NATIVE_MINT,
        connection
      );
      if (ataIx) preInstructions.push(ataIx);

      let wrapInstructions = await wrapSolIfNeeded(
        publicKey,
        publicKey,
        connection,
        collatNum
      );
      if (wrapInstructions) {
        preInstructions.push(...wrapInstructions);
      }
    }

    let collateral = new BN(collatNum * 10 ** custody.decimals);

    methodBuilder = perpetual_program.methods
      .addCollateral({
        collateral,
      })
      .accounts({
        owner: publicKey,
        fundingAccount: userCustodyTokenAccount, // user token account for custody token account
        transferAuthority: TRANSFER_AUTHORITY,
        perpetuals: PERPETUALS_ADDRESS,
        pool: pool.address,
        position: position.address,
        custody: custody.address,
        custodyOracleAccount: custody.oracle.oracleAccount,
        custodyTokenAccount: custody.tokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      });
  } else {
    if (position.token == TokenE.SOL) {
      let ataIx = await createAtaIfNeeded(
        publicKey,
        publicKey,
        NATIVE_MINT,
        connection
      );
      if (ataIx) preInstructions.push(ataIx);
    }

    // Decimals are 6, since collateral is in USD
    let collateralUsd = new BN(collatNum * 10 ** 6);
    methodBuilder = perpetual_program.methods
      .removeCollateral({
        collateralUsd,
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
      });
  }

  if (preInstructions)
    methodBuilder = methodBuilder.preInstructions(preInstructions);

  if (position.token == TokenE.SOL)
    methodBuilder = methodBuilder.postInstructions(postInstructions);

  try {
    let tx = await methodBuilder.transaction();
    await manualSendTransaction(
      tx,
      publicKey,
      connection,
      walletContextState.signTransaction
    );
    // await automaticSendTransaction(
    //   methodBuilder,
    //   perpetual_program.provider.connection
    // );
  } catch (err) {
    console.log(err);
    throw err;
  }
}
