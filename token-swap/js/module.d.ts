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
  export const CurveType: Object;

  export class TokenSwap {
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
      tradeFeeNumerator: Numberu64,
      tradeFeeDenominator: Numberu64,
      ownerTradeFeeNumerator: Numberu64,
      ownerTradeFeeDenominator: Numberu64,
      ownerWithdrawFeeNumerator: Numberu64,
      ownerWithdrawFeeDenominator: Numberu64,
      hostFeeNumerator: Numberu64,
      hostFeeDenominator: Numberu64,
      curveType: number,
      payer: Account,
    );

    static getMinBalanceRentForExemptTokenSwap(
      connection: Connection,
    ): Promise<number>;

    static createInitSwapInstruction(
      tokenSwapAccount: Account,
      authority: PublicKey,
      tokenAccountA: PublicKey,
      tokenAccountB: PublicKey,
      tokenPool: PublicKey,
      feeAccount: PublicKey,
      tokenAccountPool: PublicKey,
      tokenProgramId: PublicKey,
      swapProgramId: PublicKey,
      nonce: number,
      tradeFeeNumerator: number,
      tradeFeeDenominator: number,
      ownerTradeFeeNumerator: number,
      ownerTradeFeeDenominator: number,
      ownerWithdrawFeeNumerator: number,
      ownerWithdrawFeeDenominator: number,
      hostFeeNumerator: number,
      hostFeeDenominator: number,
      curveType: number,
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
      poolToken: PublicKey,
      mintA: PublicKey,
      mintB: PublicKey,
      feeAccount: PublicKey,
      tokenAccountPool: PublicKey,
      swapProgramId: PublicKey,
      tokenProgramId: PublicKey,
      nonce: number,
      tradeFeeNumerator: number,
      tradeFeeDenominator: number,
      ownerTradeFeeNumerator: number,
      ownerTradeFeeDenominator: number,
      ownerWithdrawFeeNumerator: number,
      ownerWithdrawFeeDenominator: number,
      hostFeeNumerator: number,
      hostFeeDenominator: number,
      curveType: number,
    ): Promise<TokenSwap>;

    swap(
      userSource: PublicKey,
      poolSource: PublicKey,
      poolDestination: PublicKey,
      userDestination: PublicKey,
      hostFeeAccount: PublicKey | null,
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
      hostFeeAccount: PublicKey | null,
      swapProgramId: PublicKey,
      tokenProgramId: PublicKey,
      amountIn: number | Numberu64,
      minimumAmountOut: number | Numberu64,
    ): TransactionInstruction;

    depositAllTokenTypes(
      userAccountA: PublicKey,
      userAccountB: PublicKey,
      poolAccount: PublicKey,
      poolTokenAmount: number | Numberu64,
      maximumTokenA: number | Numberu64,
      maximumTokenB: number | Numberu64,
    ): Promise<TransactionSignature>;

    static depositAllTokenTypesInstruction(
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

    withdrawAllTokenTypes(
      userAccountA: PublicKey,
      userAccountB: PublicKey,
      poolTokenAmount: number | Numberu64,
      minimumTokenA: number | Numberu64,
      minimumTokenB: number | Numberu64,
    ): Promise<TransactionSignature>;

    static withdrawAllTokenTypesInstruction(
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

    depositSingleTokenTypeExactAmountIn(
      userAccount: PublicKey,
      poolAccount: PublicKey,
      sourceTokenAmount: number | Numberu64,
      minimumPoolTokenAmount: number | Numberu64,
    ): Promise<TransactionSignature>;

    static depositSingleTokenTypeExactAmountInInstruction(
      tokenSwap: PublicKey,
      authority: PublicKey,
      source: PublicKey,
      intoA: PublicKey,
      intoB: PublicKey,
      poolToken: PublicKey,
      poolAccount: PublicKey,
      swapProgramId: PublicKey,
      tokenProgramId: PublicKey,
      sourceTokenAmount: number | Numberu64,
      minimumPoolTokenAmount: number | Numberu64,
    ): TransactionInstruction;

    withdrawSingleTokenTypeExactAmountOut(
      userAccount: PublicKey,
      poolAccount: PublicKey,
      destinationTokenAmount: number | Numberu64,
      maximumPoolTokenAmount: number | Numberu64,
    ): Promise<TransactionSignature>;

    static withdrawSingleTokenTypeExactAmountOutInstruction(
      tokenSwap: PublicKey,
      authority: PublicKey,
      poolMint: PublicKey,
      feeAccount: PublicKey,
      sourcePoolAccount: PublicKey,
      fromA: PublicKey,
      fromB: PublicKey,
      userAccount: PublicKey,
      swapProgramId: PublicKey,
      tokenProgramId: PublicKey,
      destinationTokenAmount: number | Numberu64,
      maximumPoolTokenAmount: number | Numberu64,
    ): TransactionInstruction;
  }
}
