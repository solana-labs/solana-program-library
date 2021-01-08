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
export const LendingMarketLayout: typeof BufferLayout.Structure = BufferLayout.struct(
  [
    BufferLayout.u8("version"),
    Layout.publicKey("quoteTokenMint"),
    Layout.publicKey("tokenProgramId"),
    BufferLayout.blob(63, "padding"),
  ]
);

export type CreateLendingMarketParams = {
  connection: Connection;
  tokenProgramId?: PublicKey;
  lendingProgramId: PublicKey;
  lendingMarketAccount: Account;
  quoteTokenMint: PublicKey;
  payer: Account;
};

export class LendingMarket {
  account: Account;
  connection: Connection;
  quoteTokenMint: PublicKey;
  tokenProgramId: PublicKey;
  lendingProgramId: PublicKey;
  payer: Account;

  constructor(params: CreateLendingMarketParams) {
    this.account = params.lendingMarketAccount;
    this.connection = params.connection;
    this.quoteTokenMint = params.quoteTokenMint;
    this.tokenProgramId = params.tokenProgramId || TOKEN_PROGRAM_ID;
    this.lendingProgramId = params.lendingProgramId;
    this.payer = params.payer;
  }

  static async create(
    params: CreateLendingMarketParams
  ): Promise<LendingMarket> {
    const lendingMarket = new LendingMarket(params);

    // Allocate memory for the account
    const balanceNeeded = await LendingMarket.getMinBalanceRentForExemptTokenReserve(
      lendingMarket.connection
    );

    const transaction = new Transaction()
      .add(
        SystemProgram.createAccount({
          fromPubkey: params.payer.publicKey,
          newAccountPubkey: lendingMarket.account.publicKey,
          lamports: balanceNeeded,
          space: LendingMarketLayout.span,
          programId: params.lendingProgramId,
        })
      )
      .add(LendingMarket.createInitLendingMarketInstruction(lendingMarket));

    await sendAndConfirmTransaction(
      lendingMarket.connection,
      transaction,
      [lendingMarket.payer, lendingMarket.account],
      { commitment: "singleGossip", preflightCommitment: "singleGossip" }
    );

    return lendingMarket;
  }

  /**
   * Get the minimum balance for the lending market account to be rent exempt
   *
   * @return Number of lamports required
   */
  static async getMinBalanceRentForExemptTokenReserve(
    connection: Connection
  ): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(
      LendingMarketLayout.span
    );
  }

  static createInitLendingMarketInstruction(
    lendingMarket: LendingMarket
  ): TransactionInstruction {
    const programId = lendingMarket.lendingProgramId;
    const keys = [
      {
        pubkey: lendingMarket.account.publicKey,
        isSigner: false,
        isWritable: true,
      },
      {
        pubkey: lendingMarket.quoteTokenMint,
        isSigner: false,
        isWritable: false,
      },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
      {
        pubkey: lendingMarket.tokenProgramId,
        isSigner: false,
        isWritable: false,
      },
    ];
    const commandDataLayout = BufferLayout.struct([
      BufferLayout.u8("instruction"),
    ]);
    let data = Buffer.alloc(1024);
    {
      const encodeLength = commandDataLayout.encode(
        {
          instruction: 0, // InitLendingMarket instruction
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
