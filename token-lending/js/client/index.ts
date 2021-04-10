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
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import * as BufferLayout from "buffer-layout";
import * as Layout from "./layout";

export const LENDING_PROGRAM_ID = new PublicKey(
  "LendZqTs7gn5CTSJU1jWKhKuVpjJGom45nnwPb2AMTi"
);

/**
 * @private
 */
export const LendingMarketLayout: typeof BufferLayout.Structure = BufferLayout.struct(
  [
    BufferLayout.u8("version"),
    BufferLayout.u8("bumpSeed"),
    Layout.publicKey("owner"),
    Layout.publicKey("quoteTokenMint"),
    Layout.publicKey("tokenProgramId"),
    BufferLayout.blob(62, "padding"),
  ]
);

export type CreateLendingMarketParams = {
  connection: Connection;
  tokenProgramId?: PublicKey;
  lendingProgramId: PublicKey;
  lendingMarketAccount: Account;
  lendingMarketOwner: PublicKey;
  quoteTokenMint: PublicKey;
  payer: Account;
};

export class LendingMarket {
  account: Account;
  connection: Connection;
  owner: PublicKey;
  quoteTokenMint: PublicKey;
  tokenProgramId: PublicKey;
  lendingProgramId: PublicKey;
  payer: Account;

  constructor(params: CreateLendingMarketParams) {
    this.account = params.lendingMarketAccount;
    this.connection = params.connection;
    this.owner = params.lendingMarketOwner;
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
      Layout.publicKey("owner"),
    ]);
    let data = Buffer.alloc(1024);
    {
      const encodeLength = commandDataLayout.encode(
        {
          instruction: 0, // InitLendingMarket instruction
          owner: lendingMarket.owner.toBuffer(),
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
