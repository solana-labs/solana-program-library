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

  declare export var TokenSwapLayout: Layout;

  declare export var CurveType: Object;

  declare export class TokenSwap {
    constructor(
      connection: Connection,
      swapProgramId: PublicKey,
      tokenProgramId: PublicKey,
      tokenSwap: PublicKey,
      poolToken: PublicKey,
      feeAccount: PublicKey,
      authority: PublicKey,
      tokenAccountA: PublicKey,
      tokenAccountB: PublicKey,
      mintA: PublicKey,
      mintB: PublicKey,
      curveType: number,
      tradeFeeNumerator: Numberu64,
      tradeFeeDenominator: Numberu64,
      ownerTradeFeeNumerator: Numberu64,
      ownerTradeFeeDenominator: Numberu64,
      ownerWithdrawFeeNumerator: Numberu64,
      ownerWithdrawFeeDenominator: Numberu64,
      hostFeeNumerator: Numberu64,
      hostFeeDenominator: Numberu64,
      payer: Account,
    ): TokenSwap;

    static getMinBalanceRentForExemptTokenSwap(
      connection: Connection,
    ): Promise<number>;

    static createInitSwapInstruction(
      programId: PublicKey,
      tokenSwapAccount: Account,
      authority: PublicKey,
      tokenAccountA: PublicKey,
      tokenAccountB: PublicKey,
      tokenPool: PublicKey,
      feeAccount: PublicKey,
      tokenAccountPool: PublicKey,
      tokenProgramId: PublicKey,
      nonce: number,
      curveType: number,
      tradeFeeNumerator: number,
      tradeFeeDenominator: number,
      ownerTradeFeeNumerator: number,
      ownerTradeFeeDenominator: number,
      ownerWithdrawFeeNumerator: number,
      ownerWithdrawFeeDenominator: number,
      hostFeeNumerator: number,
      hostFeeDenominator: number,
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
      mintA: PublicKey,
      mintB: PublicKey,
      feeAccount: PublicKey,
      tokenAccountPool: PublicKey,
      tokenProgramId: PublicKey,
      nonce: number,
      curveType: number,
      tradeFeeNumerator: number,
      tradeFeeDenominator: number,
      ownerTradeFeeNumerator: number,
      ownerTradeFeeDenominator: number,
      ownerWithdrawFeeNumerator: number,
      ownerWithdrawFeeDenominator: number,
      hostFeeNumerator: number,
      hostFeeDenominator: number,
      programId: PublicKey,
    ): Promise<TokenSwap>;

    swap(
      userSource: PublicKey,
      poolSource: PublicKey,
      poolDestination: PublicKey,
      userDestination: PublicKey,
      hostFeeAccount: ?PublicKey,
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
      poolMint: PublicKey,
      feeAccount: PublicKey,
      hostFeeAccount: ?PublicKey,
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
      userAccountA: PublicKey,
      userAccountB: PublicKey,
      poolAccount: PublicKey,
      poolTokenAmount: number | Numberu64,
      minimumTokenA: number | Numberu64,
      minimumTokenB: number | Numberu64,
    ): Promise<TransactionSignature>;

    static withdrawInstruction(
      tokenSwap: PublicKey,
      authority: PublicKey,
      poolMint: PublicKey,
      feeAccount: PublicKey,
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
