import { CustodyAccount } from "@/lib/CustodyAccount";
import { PoolAccount } from "@/lib/PoolAccount";
import { PositionAccount } from "@/lib/PositionAccount";
import { Side } from "@/lib/types";
import { IDL, Perpetuals } from "@/target/types/perpetuals";
import {
  DEFAULT_PERPS_USER,
  getPerpetualProgramAndProvider,
  PERPETUALS_ADDRESS,
  PERPETUALS_PROGRAM_ID,
} from "@/utils/constants";
import { IdlCoder } from "@/utils/IdlCoder";
import { AnchorProvider, BN, Program } from "@project-serum/anchor";
import { decode } from "@project-serum/anchor/dist/cjs/utils/bytes/base64";
import {
  Connection,
  PublicKey,
  RpcResponseAndContext,
  SimulatedTransactionResponse,
  Transaction,
} from "@solana/web3.js";

export type PositionSide = "long" | "short";

export interface PriceAndFee {
  liquidationPrice: BN;
  entryPrice: BN;
  fee: BN;
}

export interface ProfitAndLoss {
  profit: BN;
  loss: BN;
}

export interface SwapAmountAndFees {
  amountOut: BN;
  feeIn: BN;
  feeOut: BN;
}

export interface AddLiquidityAmountAndFees {
  amount: BN;
  fee: BN;
}

export interface RemoveLiquidityAmountAndFees {
  amount: BN;
  fee: BN;
}

export class ViewHelper {
  program: Program<Perpetuals>;
  connection: Connection;
  provider: AnchorProvider;
  //   poolConfig: PoolConfig;

  constructor(connection: Connection, provider: AnchorProvider) {
    this.connection = connection;
    this.provider = provider;
    this.program = new Program(IDL, PERPETUALS_PROGRAM_ID, provider);
    // this.poolConfig = PoolConfig.fromIdsByName(DEFAULT_POOL, CLUSTER);
  }

  // may need to add blockhash and also probably use VersionedTransactions
  simulateTransaction = async (
    transaction: Transaction
  ): Promise<RpcResponseAndContext<SimulatedTransactionResponse>> => {
    transaction.feePayer = DEFAULT_PERPS_USER.publicKey;
    return this.connection.simulateTransaction(transaction);
  };

  decodeLogs<T>(
    data: RpcResponseAndContext<SimulatedTransactionResponse>,
    instructionNumber: number
  ): T {
    const returnPrefix = `Program return: ${PERPETUALS_PROGRAM_ID} `;
    // console.log("Data:", data);
    if (data.value.logs && data.value.err === null) {
      let returnLog = data.value.logs.find((l: any) =>
        l.startsWith(returnPrefix)
      );
      if (!returnLog) {
        throw new Error("View expected return log");
      }
      let returnData = decode(returnLog.slice(returnPrefix.length));
      // @ts-ignore
      let returnType = IDL.instructions[instructionNumber].returns;

      if (!returnType) {
        throw new Error("View expected return type");
      }
      const coder = IdlCoder.fieldLayout(
        { type: returnType },
        Array.from([...(IDL.accounts ?? []), ...(IDL.types ?? [])])
      );
      // console.log("coder.decode(returnData); ::: ", coder.decode(returnData));
      return coder.decode(returnData);
    } else {
      throw new Error(`No Logs Found `, { cause: data });
    }
  }

  getAssetsUnderManagement = async (pool: PoolAccount): Promise<BN> => {
    let program = new Program(IDL, PERPETUALS_PROGRAM_ID, this.provider);

    const transaction = await program.methods
      // @ts-ignore
      .getAssetsUnderManagement({})
      .accounts({
        perpetuals: PERPETUALS_ADDRESS,
        pool: pool.address,
      })
      .remainingAccounts(pool.getCustodyMetas())
      .transaction();

    const result = await this.simulateTransaction(transaction);
    // console.log("result in aum fetch", result);
    const index = IDL.instructions.findIndex(
      (f) => f.name === "getAssetsUnderManagement"
    );
    return this.decodeLogs(result, index);
  };

  getEntryPriceAndFee = async (
    payAmount: number,
    positionAmount: number,
    side: Side,
    pool: PoolAccount,
    custody: CustodyAccount
  ): Promise<PriceAndFee> => {
    let program = new Program(IDL, PERPETUALS_PROGRAM_ID, this.provider);

    let collateral = new BN(payAmount * 10 ** custody.decimals);
    let size = new BN(positionAmount * 10 ** custody.decimals);

    console.log("entry params", payAmount, positionAmount, side);
    let transaction: Transaction = await program.methods
      // @ts-ignore
      .getEntryPriceAndFee({
        collateral,
        size,
        side: side === "Long" ? { long: {} } : { short: {} },
      })
      .accounts({
        perpetuals: PERPETUALS_ADDRESS,
        pool: pool.address,
        custody: custody.address,
        custodyOracleAccount: custody.oracle.oracleAccount,
      })
      .transaction();

    const result = await this.simulateTransaction(transaction);
    console.log("got entry result", result);
    const index = IDL.instructions.findIndex(
      (f) => f.name === "getEntryPriceAndFee"
    );
    const res: any = this.decodeLogs(result, index);
    console.log("res in entry price and fee", res);

    return {
      liquidationPrice: res.liquidationPrice,
      entryPrice: res.entryPrice,
      fee: res.fee,
    };
  };

  getExitPriceAndFee = async (position: PublicKey): Promise<PriceAndFee> => {
    let program = new Program(IDL, PERPETUALS_PROGRAM_ID, this.provider);
    // console.log("fee payer : ", DEFAULT_PERPS_USER.publicKey.toBase58());

    const transaction = await program.methods
      // @ts-ignore
      .getExitPriceAndFee({})
      .accounts({
        perpetuals: PERPETUALS_ADDRESS,
        pool: position.pool,
        position: position.address,
        custody: position.custody,
        custodyOracleAccount: position.oracleAccount,
      })
      .transaction();

    const result = await this.simulateTransaction(transaction);
    const index = IDL.instructions.findIndex(
      (f) => f.name === "getExitPriceAndFee"
    );
    const res: any = this.decodeLogs(result, index);

    return {
      price: res.price,
      fee: res.fee,
    };
  };

  getLiquidationPrice = async (
    position: PositionAccount,
    custody?: CustodyAccount,
    addCollat?: number,
    removeCollat?: number
  ): Promise<BN> => {
    let program = new Program(IDL, PERPETUALS_PROGRAM_ID, this.provider);

    let addCollateral = addCollat
      ? new BN(addCollat * 10 ** custody.decimals)
      : new BN(0);
    let removeCollateral = removeCollat
      ? new BN(removeCollat * 10 ** 6)
      : new BN(0);
    let params = {};

    if (addCollateral > 0 || removeCollateral > 0) {
      params = { addCollateral, removeCollateral };
    }

    console.log("\n\n\n getLiquidationPrice");
    console.log(
      "final params",
      Number(params["addCollateral"]),
      Number(params["removeCollateral"])
    );
    const transaction = await program.methods
      // @ts-ignore
      .getLiquidationPrice(params)
      .accounts({
        perpetuals: PERPETUALS_ADDRESS,
        pool: position.pool,
        position: position.address,
        custody: position.custody,
        custodyOracleAccount: position.oracleAccount,
      })
      .transaction();

    const result = await this.simulateTransaction(transaction);

    const index = IDL.instructions.findIndex(
      (f) => f.name === "getLiquidationPrice"
    );
    console.log("results in liquidation price", result);
    console.log("decode logs", Number(this.decodeLogs(result, index)));
    console.log("\n\n\n***");
    return this.decodeLogs(result, index);
  };

  getLiquidationState = async (position: PositionAccount): Promise<BN> => {
    let program = new Program(IDL, PERPETUALS_PROGRAM_ID, this.provider);

    const transaction = await program.methods
      // @ts-ignore
      .getLiquidationState({})
      .accounts({
        perpetuals: PERPETUALS_ADDRESS,
        pool: position.pool,
        position: position.address,
        custody: position.custody,
        custodyOracleAccount: position.oracleAccount,
      })
      .transaction();

    const result = await this.simulateTransaction(transaction);
    const index = IDL.instructions.findIndex(
      (f) => f.name === "getLiquidationState"
    );
    return this.decodeLogs(result, index);
  };

  getPnl = async (position: PositionAccount): Promise<ProfitAndLoss> => {
    let { perpetual_program } = await getPerpetualProgramAndProvider();
    const transaction = await perpetual_program.methods
      .getPnl({})
      .accounts({
        perpetuals: PERPETUALS_ADDRESS,
        pool: position.pool,
        position: position.address,
        custody: position.custody,
        custodyOracleAccount: position.oracleAccount,
      })
      .transaction();

    const result = await this.simulateTransaction(transaction);
    const index = IDL.instructions.findIndex((f) => f.name === "getPnl");
    const res: any = this.decodeLogs<BN>(result, index);
    return {
      profit: res.profit,
      loss: res.loss,
    };
  };

  getSwapAmountAndFees = async (
    amtIn: number,
    pool: PoolAccount,
    receivingCustody: CustodyAccount,
    dispensingCustody: CustodyAccount
  ): Promise<SwapAmountAndFees> => {
    let program = new Program(IDL, PERPETUALS_PROGRAM_ID, this.provider);
    let amountIn = new BN(amtIn * 10 ** receivingCustody.decimals);

    // console.log("amount in", Number(amountIn));
    let transaction = await program.methods
      // @ts-ignore
      .getSwapAmountAndFees({
        amountIn,
      })
      .accounts({
        perpetuals: PERPETUALS_ADDRESS,
        pool: pool.address,
        receivingCustody: receivingCustody.address,
        receivingCustodyOracleAccount: receivingCustody.oracle.oracleAccount,
        dispensingCustody: dispensingCustody.address,
        dispensingCustodyOracleAccount: dispensingCustody.oracle.oracleAccount,
      })
      .transaction();

    const result = await this.simulateTransaction(transaction);
    // console.log("result in swap  fetch", result);
    const index = IDL.instructions.findIndex(
      (f) => f.name === "getSwapAmountAndFees"
    );
    const res: any = this.decodeLogs(result, index);

    return {
      amountOut: res.amountOut,
      feeIn: res.feeIn,
      feeOut: res.feeOut,
    };
  };

  getAddLiquidityAmountAndFees = async (
    amtIn: number,
    pool: PoolAccount,
    custody: CustodyAccount
  ): Promise<AddLiquidityAmountAndFees> => {
    let program = new Program(IDL, PERPETUALS_PROGRAM_ID, this.provider);
    let amountIn = new BN(amtIn * 10 ** custody.decimals);
    let transaction = await program.methods
      .getAddLiquidityAmountAndFee({
        amountIn,
      })
      .accounts({
        perpetuals: PERPETUALS_ADDRESS,
        pool: pool.address,
        custody: custody.address,
        custodyOracleAccount: custody.oracle.oracleAccount,
        lpTokenMint: pool.getLpTokenMint(),
      })
      .remainingAccounts(pool.getCustodyMetas())
      .transaction();
    const result = await this.simulateTransaction(transaction);

    // console.log("result", result);
    const index = IDL.instructions.findIndex(
      (f) => f.name === "getAddLiquidityAmountAndFee"
    );
    const res: any = this.decodeLogs(result, index);
    return {
      amount: res.amount,
      fee: res.fee,
    };
  };

  getRemoveLiquidityAmountAndFees = async (
    lpIn: number,
    pool: PoolAccount,
    custody: CustodyAccount
  ): Promise<RemoveLiquidityAmountAndFees> => {
    let program = new Program(IDL, PERPETUALS_PROGRAM_ID, this.provider);

    let lpAmountIn = new BN(lpIn * 10 ** pool.lpData.decimals);

    let transaction = await program.methods
      .getRemoveLiquidityAmountAndFee({
        lpAmountIn,
      })
      .accounts({
        perpetuals: PERPETUALS_ADDRESS,
        pool: pool.address,
        custody: custody.address,
        custodyOracleAccount: custody.oracle.oracleAccount,
        lpTokenMint: pool.getLpTokenMint(),
      })
      .remainingAccounts(pool.getCustodyMetas())
      .transaction();
    const result = await this.simulateTransaction(transaction);
    const index = IDL.instructions.findIndex(
      (f) => f.name === "getRemoveLiquidityAmountAndFee"
    );
    const res: any = this.decodeLogs(result, index);
    return {
      amount: res.amount,
      fee: res.fee,
    };
  };

  getOraclePrice = async (
    pool: PoolAccount,
    ema: boolean,
    custody: CustodyAccount
  ): Promise<BN> => {
    const transaction = await this.program.methods
      .getOraclePrice({
        ema,
      })
      .accounts({
        perpetuals: PERPETUALS_ADDRESS,
        pool: pool.address,
        custody: custody.address,
        custodyOracleAccount: custody.oracle.oracleAccount,
      })
      .transaction();

    const result = await this.simulateTransaction(transaction);
    // console.log("oracle result", result);
    const index = IDL.instructions.findIndex(
      (f) => f.name === "getOraclePrice"
    );
    return this.decodeLogs<BN>(result, index);
  };
}
