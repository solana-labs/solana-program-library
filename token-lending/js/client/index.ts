/* eslint-disable @typescript-eslint/no-unsafe-assignment */
/* eslint-disable @typescript-eslint/no-unsafe-call */
/* eslint-disable @typescript-eslint/no-unsafe-member-access */

import {
  Account,
  Connection,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  SYSVAR_RENT_PUBKEY,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import * as BufferLayout from "buffer-layout";
import * as Layout from "./layout";

const TOKEN_PROGRAM_ID = new PublicKey(
  "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
);

/**
 * @private
 */
export const LendingPoolLayout: typeof BufferLayout.Structure = BufferLayout.struct(
  [
    BufferLayout.u8("isInitialized"),
    Layout.publicKey("quoteTokenMint"),
    BufferLayout.u8("numReserves"),
    Layout.publicKey('reserve1'),
    Layout.publicKey('reserve2'),
    Layout.publicKey('reserve3'),
    Layout.publicKey('reserve4'),
    Layout.publicKey('reserve5'),
    Layout.publicKey('reserve6'),
    Layout.publicKey('reserve7'),
    Layout.publicKey('reserve8'),
    Layout.publicKey('reserve9'),
    Layout.publicKey('reserve10'),
  ]
);

/**
 * @private
 */
export const PoolReserveLayout: typeof BufferLayout.Structure = BufferLayout.struct(
  [
    BufferLayout.u8("isInitialized"),
    Layout.publicKey("pool"),
    Layout.publicKey("reserveToken"),
    Layout.publicKey("collateralToken"),
    Layout.publicKey("liquidityTokenMint"),
    Layout.publicKey("dexMarket"),
    BufferLayout.u64("marketPrice"),
    BufferLayout.u64("marketPriceUpdatedSlot"),
  ]
);

/**
 * @private
 */
export const BorrowObligationLayout: typeof BufferLayout.Structure = BufferLayout.struct(
  [
    BufferLayout.u64("createdAtSlot"),
    Layout.publicKey("authority"),
    BufferLayout.u64("collateralAmount"),
    Layout.publicKey("collateralReserve"),
    BufferLayout.u64("borrowAmount"),
    Layout.publicKey("borrowReserve"),
  ]
);

export type TokenLendingPoolParams = {
  connection: Connection;
  tokenProgramId?: PublicKey;
  lendingProgramId?: PublicKey;
  quoteTokenMint: PublicKey;
  reserves: Array<PoolReserve>;
  payer: Account;
};

export class TokenLendingPool {
  connection: Connection;
  reserves: Array<PoolReserve>;
  quoteTokenMint: PublicKey;

  constructor(params: TokenLendingPoolParams) {
    this.connection = params.connection;
    this.reserves = params.reserves;
    this.quoteTokenMint = params.quoteTokenMint;
  }
}

export type PoolReserveParams = {
  connection: Connection;
  tokenProgramId?: PublicKey;
  lendingProgramId: PublicKey;
  reserveAccount: Account;
  reserveToken: PublicKey;
  collateralToken: PublicKey;
  liquidityTokenMint: PublicKey;
  payer: Account;
};

export type InitReserveInstructionParams = {
  reserveAccount: PublicKey;
  reserveToken: PublicKey;
  collateralToken: PublicKey;
  liquidityTokenMint: PublicKey;
  tokenProgramId?: PublicKey;
  lendingProgramId: PublicKey;
};

export class PoolReserve {
  connection: Connection;
  tokenProgramId: PublicKey;
  lendingProgramId: PublicKey;
  reserveAccount: Account;
  reserveToken: PublicKey;
  collateralToken: PublicKey;
  liquidityTokenMint: PublicKey;
  payer: Account;

  constructor(params: PoolReserveParams) {
    this.connection = params.connection;
    this.tokenProgramId = params.tokenProgramId || TOKEN_PROGRAM_ID;
    this.lendingProgramId = params.lendingProgramId;
    this.reserveAccount = params.reserveAccount;
    this.reserveToken = params.reserveToken;
    this.collateralToken = params.collateralToken;
    this.liquidityTokenMint = params.liquidityTokenMint;
    this.payer = params.payer;
  }

  static async create(params: PoolReserveParams): Promise<PoolReserve> {
    const poolReserve = new PoolReserve(params);

    // Allocate memory for the account
    const balanceNeeded = await PoolReserve.getMinBalanceRentForExemptPoolReserve(
      poolReserve.connection
    );

    const transaction = new Transaction()
      .add(
        SystemProgram.createAccount({
          fromPubkey: poolReserve.payer.publicKey,
          newAccountPubkey: poolReserve.reserveAccount.publicKey,
          lamports: balanceNeeded,
          space: PoolReserveLayout.span,
          programId: poolReserve.lendingProgramId,
        })
      )
      .add(
        await PoolReserve.createInitReserveInstruction({
          ...poolReserve,
          reserveAccount: poolReserve.reserveAccount.publicKey,
        })
      );

    await sendAndConfirmTransaction(
      poolReserve.connection,
      transaction,
      [poolReserve.payer, poolReserve.reserveAccount],
      { commitment: "singleGossip", preflightCommitment: "singleGossip" }
    );

    return poolReserve;
  }

  /**
   * Get the minimum balance for the token reserve account to be rent exempt
   *
   * @return Number of lamports required
   */
  static async getMinBalanceRentForExemptPoolReserve(
    connection: Connection
  ): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(
      PoolReserveLayout.span
    );
  }

  static async createInitReserveInstruction(
    params: InitReserveInstructionParams
  ): Promise<TransactionInstruction> {
    const tokenProgramId = params.tokenProgramId || TOKEN_PROGRAM_ID;
    const programId = params.lendingProgramId;
    const keys = [
      { pubkey: params.reserveAccount, isSigner: false, isWritable: true },
      { pubkey: params.reserveToken, isSigner: false, isWritable: false },
      { pubkey: params.collateralToken, isSigner: false, isWritable: false },
      { pubkey: params.liquidityTokenMint, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: tokenProgramId, isSigner: false, isWritable: false },
    ];
    const commandDataLayout = BufferLayout.struct([
      BufferLayout.u8("instruction"),
      Layout.publicKey("authority"),
    ]);
    const [authority] = await PublicKey.findProgramAddress(
      [params.reserveAccount.toBuffer()],
      programId
    );
    let data = Buffer.alloc(1024);
    {
      const encodeLength = commandDataLayout.encode(
        {
          instruction: 0, // InitializeReserve instruction
          authority: authority.toBuffer(),
        },
        data
      );
      data = data.slice(0, encodeLength);
    }
    return new TransactionInstruction({
      keys,
      programId,
      data,
    });
  }
}
