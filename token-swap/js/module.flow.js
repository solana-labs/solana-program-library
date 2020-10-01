/**
 * Flow Library definition for spl-token-swap
 *
 * This file is manually maintained
 *
 */

declare module '@solana/spl-token-swap' {
  // === client/token-swap.js ===
  declare export class Numberu64 extends BN {
    toBuffer(): Buffer;
    static fromBuffer(buffer: Buffer): Numberu64;
  }

  declare export type TokenSwapInfo = {|
    nonce: number,
    tokenAccountA: PublicKey,
    tokenAccountB: PublicKey,
    tokenPool: PublicKey,
    feesNumerator: Numberu64,
    feesDenominator: Numberu64,
    feeRatio: number,
  |};

  declare export var TokenSwapLayout: Layout;

  declare export class TokenSwap {
    constructor(
      connection: Connection,
      tokenSwap: PublicKey,
      programId: PublicKey,
      payer: Account,
    ): TokenSwap;

    static getMinBalanceRentForExemptTokenSwap(
      connection: Connection,
    ): Promise<number>;

    static createInitSwapInstruction(
      programId: PublicKey,
      tokenSwapAccount: Account,
      authority: PublicKey,
      nonce: number,
      tokenAccountA: PublicKey,
      tokenAccountB: PublicKey,
      tokenPool: PublicKey,
      tokenAccountPool: PublicKey,
      tokenProgramId: PublicKey,
      feeNumerator: number,
      feeDenominator: number,
    ): TransactionInstruction;

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
      programId: PublicKey,
    ): Promise<TokenSwap>;

    getInfo(): Promise<TokenSwapInfo>;

    swap(
      authority: PublicKey,
      source: PublicKey,
      swapSource: PublicKey,
      swapDestination: PublicKey,
      destination: PublicKey,
      tokenProgramId: PublicKey,
      amountIn: number | Numberu64,
      minimumAmountOut: number | Numberu64,
    ): Promise<TransactionSignature>;

    static swapInstruction(
      tokenSwap: PublicKey,
      authority: PublicKey,
      source: PublicKey,
      swapSource: PublicKey,
      swapDestination: PublicKey,
      destination: PublicKey,
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
