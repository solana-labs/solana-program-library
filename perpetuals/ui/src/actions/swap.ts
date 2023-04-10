import { PoolAccount } from "@/lib/PoolAccount";
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
  wrapSolIfNeeded,
} from "@/utils/transactionHelpers";
import { BN } from "@project-serum/anchor";
import { MethodsBuilder } from "@project-serum/anchor/dist/cjs/program/namespace/methods";
import { getAssociatedTokenAddress, TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { WalletContextState } from "@solana/wallet-adapter-react";
import { Connection, TransactionInstruction } from "@solana/web3.js";

export async function swapTransactionBuilder(
  walletContextState: WalletContextState,
  connection: Connection,
  pool: PoolAccount,
  topToken: TokenE,
  bottomToken: TokenE,
  amtInNumber: number,
  minAmtOutNumber?: number
  // @ts-ignore
): Promise<{
  methodBuilder: MethodsBuilder;
  preInstructions: TransactionInstruction[];
  postInstructions: TransactionInstruction[];
}> {
  console.log("in swap builder");
  let { perpetual_program } = await getPerpetualProgramAndProvider(
    walletContextState
  );
  let publicKey = walletContextState.publicKey!;

  const receivingCustody = pool.getCustodyAccount(topToken)!;

  let fundingAccount = await getAssociatedTokenAddress(
    receivingCustody.mint,
    publicKey
  );

  const dispensingCustody = pool.getCustodyAccount(bottomToken)!;

  console.log("receiving accoutn", dispensingCustody.getTokenE());
  let receivingAccount = await getAssociatedTokenAddress(
    dispensingCustody.mint,
    publicKey
  );

  let preInstructions: TransactionInstruction[] = [];

  if (receivingCustody.getTokenE() == TokenE.SOL) {
    console.log("sending sol", receivingCustody.getTokenE());
    let ataIx = await createAtaIfNeeded(
      publicKey,
      publicKey,
      receivingCustody.mint,
      connection
    );

    if (ataIx) preInstructions.push(ataIx);

    let wrapInstructions = await wrapSolIfNeeded(
      publicKey,
      publicKey,
      connection,
      amtInNumber
    );
    if (wrapInstructions) {
      preInstructions.push(...wrapInstructions);
    }
  }

  // console.log("dispensing custody", dispensingCustody.getTokenE());
  // console.log(
  //   "dispensing ata, or receiving account",
  //   receivingAccount.toString()
  // );
  let ataIx = await createAtaIfNeeded(
    publicKey,
    publicKey,
    dispensingCustody.mint,
    connection
  );

  if (ataIx) preInstructions.push(ataIx);

  // console.log("params", minAmtOutNumber);
  console.log("/n/n/n setting min amount");
  let minAmountOut;
  // TODO explain why there is an if statement here
  if (minAmtOutNumber) {
    console.log("FIRST min amt");
    minAmountOut = new BN(minAmtOutNumber * 10 ** dispensingCustody.decimals)
      .mul(new BN(90))
      .div(new BN(100));
  } else {
    console.log("SECOND min amt");
    minAmountOut = new BN(amtInNumber * 10 ** dispensingCustody.decimals)
      .mul(new BN(90))
      .div(new BN(100));
  }

  // console.log("amt in values", amtInNumber, receivingCustody.decimals);
  let amountIn = new BN(amtInNumber * 10 ** receivingCustody.decimals);
  // console.log("min amoutn out", Number(minAmountOut));
  let postInstructions: TransactionInstruction[] = [];
  let unwrapTx = await unwrapSolIfNeeded(publicKey, publicKey, connection);
  if (unwrapTx) postInstructions.push(...unwrapTx);

  const params: any = {
    amountIn,
    minAmountOut,
  };

  console.log(
    "swap params",
    Number(params.amountIn),
    Number(params.minAmountOut)
  );

  // console.log(
  //   "amout ins",
  //   amtInNumber,
  //   Number(amountIn),
  //   dispensingCustody.decimals,
  //   dispensingCustody.getTokenE()
  // );

  let methodBuilder = perpetual_program.methods.swap(params).accounts({
    owner: publicKey,
    fundingAccount: fundingAccount,
    receivingAccount: receivingAccount,
    transferAuthority: TRANSFER_AUTHORITY,
    perpetuals: PERPETUALS_ADDRESS,
    pool: pool.address,

    receivingCustody: receivingCustody.address,
    receivingCustodyOracleAccount: receivingCustody.oracle.oracleAccount,
    receivingCustodyTokenAccount: receivingCustody.tokenAccount,

    dispensingCustody: dispensingCustody.address,
    dispensingCustodyOracleAccount: dispensingCustody.oracle.oracleAccount,
    dispensingCustodyTokenAccount: dispensingCustody.tokenAccount,

    tokenProgram: TOKEN_PROGRAM_ID,
  });

  if (preInstructions) {
    methodBuilder = methodBuilder.preInstructions(preInstructions);
  }
  if (
    dispensingCustody.getTokenE() == TokenE.SOL ||
    receivingCustody.getTokenE() == TokenE.SOL
  ) {
    methodBuilder = methodBuilder.postInstructions(postInstructions);
  }

  return { methodBuilder, preInstructions, postInstructions };
}

export async function swap(
  walletContextState: WalletContextState,
  connection: Connection,
  pool: PoolAccount,
  topToken: TokenE,
  bottomToken: TokenE,
  amtInNumber: number,
  minAmtOutNumber?: number
) {
  let { methodBuilder } = await swapTransactionBuilder(
    walletContextState,
    connection,
    pool,
    topToken,
    bottomToken,
    amtInNumber,
    minAmtOutNumber
  );

  let publicKey = walletContextState.publicKey!;
  console.log("made swap buidler in SWAP");

  try {
    // await automaticSendTransaction(methodBuilder, connection);
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
