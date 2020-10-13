declare module '@solana/spl-token-swap' {
  import {Buffer} from 'buffer';
  import {Layout} from 'buffer-layout';
  import {
    PublicKey,
    TransactionInstruction,
    TransactionSignature,
    Connection,
    Account,
  } from '@solana/web3.js';
  import BN from 'bn.js';

  // === client/token-swap.js ===
  export class Numberu64 extends BN {
    toBuffer(): Buffer;
    static fromBuffer(buffer: Buffer): Numberu64;
  }

  export const TokenSwapLayout: Layout;

  export class TokenSwap {
    constructor(
      connection: Connection,
      swapProgramId: PublicKey,
      tokenProgramId: PublicKey,
      tokenSwap: PublicKey,
      poolToken: PublicKey,
      authority: PublicKey,
      tokenAccountA: PublicKey,
      tokenAccountB: PublicKey,
      feeNumerator: Numberu64,
      feeDenominator: Numberu64,
      payer: Account,
    );

    static getMinBalanceRentForExemptTokenSwap(
      connection: Connection,
    ): Promise<number>;

    static createInitSwapInstruction(
      tokenSwapAccount: Account,
      authority: PublicKey,
      nonce: number,
      tokenAccountA: PublicKey,
      tokenAccountB: PublicKey,
      tokenPool: PublicKey,
      tokenAccountPool: PublicKey,
      tokenProgramId: PublicKey,
      swapProgramId: PublicKey,
      feeNumerator: number,
      feeDenominator: number,
    ): TransactionInstruction;

    static loadTokenSwap(
      connection: Connection,
      address: PublicKey,
      programId: PublicKey,
      payer: Account,
    ): Promise<TokenSwap>;

    static createTokenSwap(
      connection: Connection,
      payer: Account,
      tokenSwapAccount: Account,
      authority: PublicKey,
      tokenAccountA: PublicKey,
      tokenAccountB: PublicKey,
      tokenPool: PublicKey,
      tokenAccountPool: PublicKey,
      tokenProgramId: PublicKey,
      nonce: number,
      feeNumerator: number,
      feeDenominator: number,
      swapProgramId: PublicKey,
    ): Promise<TokenSwap>;

    swap(
      userSource: PublicKey,
      poolSource: PublicKey,
      poolDestination: PublicKey,
      userDestination: PublicKey,
      amountIn: number | Numberu64,
      minimumAmountOut: number | Numberu64,
    ): Promise<TransactionSignature>;

    static swapInstruction(
      tokenSwap: PublicKey,
      authority: PublicKey,
      userSource: PublicKey,
      poolSource: PublicKey,
      poolDestination: PublicKey,
      userDestination: PublicKey,
      swapProgramId: PublicKey,
      tokenProgramId: PublicKey,
      amountIn: number | Numberu64,
      minimumAmountOut: number | Numberu64,
    ): TransactionInstruction;

    deposit(
      authority: PublicKey,
      sourceA: PublicKey,
      sourceB: PublicKey,
      intoA: PublicKey,
      intoB: PublicKey,
      poolToken: PublicKey,
      poolAccount: PublicKey,
      tokenProgramId: PublicKey,
      poolTokenAmount: number | Numberu64,
      maximumTokenA: number | Numberu64,
      maximumTokenB: number | Numberu64,
    ): Promise<TransactionSignature>;

    static depositInstruction(
      tokenSwap: PublicKey,
      authority: PublicKey,
      sourceA: PublicKey,
      sourceB: PublicKey,
      intoA: PublicKey,
      intoB: PublicKey,
      poolToken: PublicKey,
      poolAccount: PublicKey,
      swapProgramId: PublicKey,
      tokenProgramId: PublicKey,
      poolTokenAmount: number | Numberu64,
      maximumTokenA: number | Numberu64,
      maximumTokenB: number | Numberu64,
    ): TransactionInstruction;

    withdraw(
      authority: PublicKey,
      poolMint: PublicKey,
      sourcePoolAccount: PublicKey,
      fromA: PublicKey,
      fromB: PublicKey,
      userAccountA: PublicKey,
      userAccountB: PublicKey,
      tokenProgramId: PublicKey,
      poolTokenAmount: number | Numberu64,
      minimumTokenA: number | Numberu64,
      minimumTokenB: number | Numberu64,
    ): Promise<TransactionSignature>;

    static withdrawInstruction(
      tokenSwap: PublicKey,
      authority: PublicKey,
      poolMint: PublicKey,
      sourcePoolAccount: PublicKey,
      fromA: PublicKey,
      fromB: PublicKey,
      userAccountA: PublicKey,
      userAccountB: PublicKey,
      swapProgramId: PublicKey,
      tokenProgramId: PublicKey,
      poolTokenAmount: number | Numberu64,
      minimumTokenA: number | Numberu64,
      minimumTokenB: number | Numberu64,
    ): TransactionInstruction;
  }
}
