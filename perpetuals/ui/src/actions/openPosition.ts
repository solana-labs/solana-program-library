import { CustodyAccount } from "@/lib/CustodyAccount";
import { PoolAccount } from "@/lib/PoolAccount";
import { TokenE } from "@/lib/Token";
import { Side, TradeSide } from "@/lib/types";
import {
  automaticSendTransaction,
  manualSendTransaction,
} from "@/utils/TransactionHandlers";
import {
  PERPETUALS_ADDRESS,
  TRANSFER_AUTHORITY,
  getPerpetualProgramAndProvider,
} from "@/utils/constants";
import {
  createAtaIfNeeded,
  unwrapSolIfNeeded,
  wrapSolIfNeeded,
} from "@/utils/transactionHelpers";
import { ViewHelper } from "@/utils/viewHelpers";
import { BN } from "@project-serum/anchor";
import { findProgramAddressSync } from "@project-serum/anchor/dist/cjs/utils/pubkey";
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddress } from "@solana/spl-token";
import { WalletContextState } from "@solana/wallet-adapter-react";
import {
  Connection,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import { swapTransactionBuilder } from "src/actions/swap";

export async function openPositionBuilder(
  walletContextState: WalletContextState,
  connection: Connection,
  pool: PoolAccount,
  payCustody: CustodyAccount,
  positionCustody: CustodyAccount,
  payAmount: number,
  positionAmount: number,
  price: number,
  side: Side,
  leverage: number
) {
  // console.log("in open position");
  let { perpetual_program, provider } = await getPerpetualProgramAndProvider(
    walletContextState
  );
  let publicKey = walletContextState.publicKey!;

  // TODO: need to take slippage as param , this is now for testing
  const newPrice =
    side.toString() == "Long"
      ? new BN((price * 10 ** 6 * 115) / 100)
      : new BN((price * 10 ** 6 * 90) / 100);

  let userCustodyTokenAccount = await getAssociatedTokenAddress(
    positionCustody.mint,
    publicKey
  );

  let positionAccount = findProgramAddressSync(
    [
      Buffer.from("position"),
      publicKey.toBuffer(),
      pool.address.toBuffer(),
      positionCustody.address.toBuffer(),
      // @ts-ignore
      side.toString() == "Long" ? [1] : [2],
    ],
    perpetual_program.programId
  )[0];

  let preInstructions: TransactionInstruction[] = [];

  let finalPayAmount = positionAmount / leverage;

  if (payCustody.getTokenE() != positionCustody.getTokenE()) {
    console.log("first swapping in open pos");
    const View = new ViewHelper(connection, provider);
    let swapInfo = await View.getSwapAmountAndFees(
      payAmount,
      pool!,
      payCustody,
      positionCustody
    );

    let swapAmountOut =
      Number(swapInfo.amountOut) / 10 ** positionCustody.decimals;

    let swapFee = Number(swapInfo.feeOut) / 10 ** positionCustody.decimals;

    let recAmt = swapAmountOut - swapFee;

    console.log("rec amt in swap builder", recAmt, swapAmountOut, swapFee);

    let getEntryPrice = await View.getEntryPriceAndFee(
      recAmt,
      positionAmount,
      side,
      pool!,
      positionCustody!
    );

    let entryFee = Number(getEntryPrice.fee) / 10 ** positionCustody.decimals;

    console.log("entry fee in swap builder", entryFee);

    let swapInfo2 = await View.getSwapAmountAndFees(
      payAmount + entryFee + swapFee,
      pool!,
      payCustody,
      positionCustody
    );

    let swapAmountOut2 =
      Number(swapInfo2.amountOut) / 10 ** positionCustody.decimals -
      Number(swapInfo2.feeOut) / 10 ** positionCustody.decimals -
      entryFee;

    let extraSwap = 0;

    if (swapAmountOut2 < finalPayAmount) {
      let difference = (finalPayAmount - swapAmountOut2) / swapAmountOut2;
      extraSwap = difference * (payAmount + entryFee + swapFee);
    }

    let { methodBuilder: swapBuilder, preInstructions: swapPreInstructions } =
      await swapTransactionBuilder(
        walletContextState,
        connection,
        pool,
        payCustody.getTokenE(),
        positionCustody.getTokenE(),
        payAmount + entryFee + swapFee + extraSwap,
        recAmt
      );

    let ix = await swapBuilder.instruction();
    preInstructions.push(...swapPreInstructions, ix);
  }

  if (
    preInstructions.length == 0 &&
    positionCustody.getTokenE() == TokenE.SOL
  ) {
    let ataIx = await createAtaIfNeeded(
      publicKey,
      publicKey,
      positionCustody.mint,
      connection
    );

    if (ataIx) preInstructions.push(ataIx);

    let wrapInstructions = await wrapSolIfNeeded(
      publicKey,
      publicKey,
      connection,
      payAmount
    );
    if (wrapInstructions) {
      preInstructions.push(...wrapInstructions);
    }
  }

  let postInstructions: TransactionInstruction[] = [];
  let unwrapTx = await unwrapSolIfNeeded(publicKey, publicKey, connection);
  if (unwrapTx) postInstructions.push(...unwrapTx);

  const params: any = {
    price: newPrice,
    collateral: new BN(finalPayAmount * 10 ** positionCustody.decimals),
    size: new BN(positionAmount * 10 ** positionCustody.decimals),
    side: side.toString() == "Long" ? TradeSide.Long : TradeSide.Short,
  };

  let methodBuilder = perpetual_program.methods.openPosition(params).accounts({
    owner: publicKey,
    fundingAccount: userCustodyTokenAccount,
    transferAuthority: TRANSFER_AUTHORITY,
    perpetuals: PERPETUALS_ADDRESS,
    pool: pool.address,
    position: positionAccount,
    custody: positionCustody.address,
    custodyOracleAccount: positionCustody.oracle.oracleAccount,
    custodyTokenAccount: positionCustody.tokenAccount,
    systemProgram: SystemProgram.programId,
    tokenProgram: TOKEN_PROGRAM_ID,
  });

  if (preInstructions) {
    methodBuilder = methodBuilder.preInstructions(preInstructions);
  }

  if (
    payCustody.getTokenE() == TokenE.SOL ||
    positionCustody.getTokenE() == TokenE.SOL
  ) {
    methodBuilder = methodBuilder.postInstructions(postInstructions);
  }

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

export async function openPosition(
  walletContextState: WalletContextState,
  connection: Connection,
  pool: PoolAccount,
  payToken: TokenE,
  positionToken: TokenE,
  payAmount: number,
  positionAmount: number,
  price: number,
  side: Side,
  leverage: number
) {
  let payCustody = pool.getCustodyAccount(payToken)!;
  let positionCustody = pool.getCustodyAccount(positionToken)!;

  await openPositionBuilder(
    walletContextState,
    connection,
    pool,
    payCustody,
    positionCustody,
    payAmount,
    positionAmount,
    price,
    side,
    leverage
  );
}
