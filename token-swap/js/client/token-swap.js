/**
 * @flow
 */

import assert from 'assert';
import BN from 'bn.js';
import * as BufferLayout from 'buffer-layout';
import type {Connection, TransactionSignature} from '@solana/web3.js';
import {
  Account,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from '@solana/web3.js';

import * as Layout from './layout';
import {sendAndConfirmTransaction} from './util/send-and-confirm-transaction';
import {loadAccount} from './util/account';

/**
 * Some amount of tokens
 */
export class Numberu64 extends BN {
  /**
   * Convert to Buffer representation
   */
  toBuffer(): typeof Buffer {
    const a = super.toArray().reverse();
    const b = Buffer.from(a);
    if (b.length === 8) {
      return b;
    }
    assert(b.length < 8, 'Numberu64 too large');

    const zeroPad = Buffer.alloc(8);
    b.copy(zeroPad);
    return zeroPad;
  }

  /**
   * Construct a Numberu64 from Buffer representation
   */
  static fromBuffer(buffer: typeof Buffer): Numberu64 {
    assert(buffer.length === 8, `Invalid buffer length: ${buffer.length}`);
    return new BN(
      [...buffer]
        .reverse()
        .map(i => `00${i.toString(16)}`.slice(-2))
        .join(''),
      16,
    );
  }
}

/**
 * Information about a token swap
 */
type TokenSwapInfo = {|
  /**
   * Nonce. Used to generate the valid program address in the program
   */
  nonce: number,

  tokenProgramId: PublicKey,

  /**
   * Token A. The Liquidity token is issued against this value.
   */
  tokenAccountA: PublicKey,

  /**
   * Token B
   */
  tokenAccountB: PublicKey,

  /**
   * Pool tokens are issued when A or B tokens are deposited
   * Pool tokens can be withdrawn back to the original A or B token
   */
  tokenPool: PublicKey,

  /**
   * Fee numerator
   */
  feesNumerator: Numberu64,

  /**
   * Fee denominator
   */
  feesDenominator: Numberu64,

  /**
   * Fee ratio applied to the input token amount prior to output calculation
   */
  feeRatio: number,
|};

/**
 * @private
 */
export const TokenSwapLayout: typeof BufferLayout.Structure = BufferLayout.struct(
  [
    BufferLayout.u8('isInitialized'),
    BufferLayout.u8('nonce'),
    Layout.publicKey('tokenProgramId'),
    Layout.publicKey('tokenAccountA'),
    Layout.publicKey('tokenAccountB'),
    Layout.publicKey('tokenPool'),
    Layout.uint64('feesNumerator'),
    Layout.uint64('feesDenominator'),
  ],
);

/**
 * A program to exchange tokens against a pool of liquidity
 */
export class TokenSwap {
  /**
   * @private
   */
  connection: Connection;

  /**
   * Program Identifier for the Swap program
   */
  swapProgramId: PublicKey;

  /**
   * Program Identifier for the Token program
   */
  tokenProgramId: PublicKey;

  /**
   * The public key identifying this swap program
   */
  tokenSwap: PublicKey;

  /**
   * The public key for the liquidity pool token
   */
  poolToken: PublicKey;

  /**
   * Authority
   */
  authority: PublicKey;

  /**
   * The public key for the first token of the trading pair
   */
  tokenAccountA: PublicKey;

  /**
   * The public key for the second token of the trading pair
   */
  tokenAccountB: PublicKey;

  /**
   * Fee payer
   */
  payer: Account;

  /**
   * Create a Token object attached to the specific token
   *
   * @param connection The connection to use
   * @param tokenSwap The token swap account
   * @param swapProgramId The program ID of the token-swap program
   * @param tokenProgramId The program ID of the token program
   * @param poolToken The pool token
   * @param authority The authority over the swap and accounts
   * @param tokenAccountA: The token swap's Token A account
   * @param tokenAccountB: The token swap's Token B account
   * @param payer Pays for the transaction
   */
  constructor(
    connection: Connection,
    tokenSwap: PublicKey,
    swapProgramId: PublicKey,
    tokenProgramId: PublicKey,
    poolToken: PublicKey,
    authority: PublicKey,
    tokenAccountA: PublicKey,
    tokenAccountB: PublicKey,
    payer: Account,
  ) {
    Object.assign(this, {
      connection,
      tokenSwap,
      swapProgramId,
      tokenProgramId,
      poolToken,
      authority,
      tokenAccountA,
      tokenAccountB,
      payer,
    });
  }

  /**
   * Get the minimum balance for the token swap account to be rent exempt
   *
   * @return Number of lamports required
   */
  static async getMinBalanceRentForExemptTokenSwap(
    connection: Connection,
  ): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(
      TokenSwapLayout.span,
    );
  }

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
  ): TransactionInstruction {
    const keys = [
      {pubkey: tokenSwapAccount.publicKey, isSigner: false, isWritable: true},
      {pubkey: authority, isSigner: false, isWritable: false},
      {pubkey: tokenAccountA, isSigner: false, isWritable: false},
      {pubkey: tokenAccountB, isSigner: false, isWritable: false},
      {pubkey: tokenPool, isSigner: false, isWritable: true},
      {pubkey: tokenAccountPool, isSigner: false, isWritable: true},
      {pubkey: tokenProgramId, isSigner: false, isWritable: false},
    ];
    const commandDataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      BufferLayout.nu64('feeNumerator'),
      BufferLayout.nu64('feeDenominator'),
      BufferLayout.u8('nonce'),
    ]);
    let data = Buffer.alloc(1024);
    {
      const encodeLength = commandDataLayout.encode(
        {
          instruction: 0, // InitializeSwap instruction
          feeNumerator,
          feeDenominator,
          nonce,
        },
        data,
      );
      data = data.slice(0, encodeLength);
    }
    return new TransactionInstruction({
      keys,
      programId: swapProgramId,
      data,
    });
  }

  static async loadTokenSwapInfo(
    connection: Connection,
    address: PublicKey,
    programId: PublicKey,
  ): Promise<TokenSwapInfo> {
    const data = await loadAccount(connection, address, programId);
    const tokenSwapInfo = TokenSwapLayout.decode(data);
    if (!tokenSwapInfo.isInitialized) {
      throw new Error(`Invalid token swap state`);
    }

    return tokenSwapInfo;
  }

  static async loadTokenSwap(
    connection: Connection,
    address: PublicKey,
    programId: PublicKey,
    payer: Account,
  ): Promise<TokenSwap> {
    const tokenSwapInfo = await TokenSwap.loadTokenSwapInfo(
      connection,
      address,
      programId,
    );

    const [authority, nonce] = await PublicKey.findProgramAddress(
      [address.toBuffer()],
      programId,
    );

    const poolToken = new PublicKey(tokenSwapInfo.tokenPool);
    const tokenAccountA = new PublicKey(tokenSwapInfo.tokenAccountA);
    const tokenAccountB = new PublicKey(tokenSwapInfo.tokenAccountB);
    const tokenProgramId = new PublicKey(tokenSwapInfo.tokenProgramId);

    return new TokenSwap(
      connection,
      address,
      programId,
      tokenProgramId,
      poolToken,
      authority,
      tokenAccountA,
      tokenAccountB,
      payer,
    );
  }

  /**
   * Create a new Token Swap
   *
   * @param connection The connection to use
   * @param payer Pays for the transaction
   * @param tokenSwapAccount The token swap account
   * @param authority The authority over the swap and accounts
   * @param nonce The nonce used to generate the authority
   * @param tokenAccountA: The token swap's Token A account
   * @param tokenAccountB: The token swap's Token B account
   * @param poolToken The pool token
   * @param tokenAccountPool The token swap's pool token account
   * @param tokenProgramId The program ID of the token program
   * @param swapProgramId The program ID of the token-swap program
   * @param feeNumerator Numerator of the fee ratio
   * @param feeDenominator Denominator of the fee ratio
   * @return Token object for the newly minted token, Public key of the account holding the total supply of new tokens
   */
  static async createTokenSwap(
    connection: Connection,
    payer: Account,
    tokenSwapAccount: Account,
    authority: PublicKey,
    nonce: number,
    tokenAccountA: PublicKey,
    tokenAccountB: PublicKey,
    poolToken: PublicKey,
    tokenAccountPool: PublicKey,
    swapProgramId: PublicKey,
    tokenProgramId: PublicKey,
    feeNumerator: number,
    feeDenominator: number,
  ): Promise<TokenSwap> {
    let transaction;
    const tokenSwap = new TokenSwap(
      connection,
      tokenSwapAccount.publicKey,
      swapProgramId,
      tokenProgramId,
      poolToken,
      authority,
      tokenAccountA,
      tokenAccountB,
      payer,
    );

    // Allocate memory for the account
    const balanceNeeded = await TokenSwap.getMinBalanceRentForExemptTokenSwap(
      connection,
    );
    transaction = new Transaction();
    transaction.add(
      SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: tokenSwapAccount.publicKey,
        lamports: balanceNeeded,
        space: TokenSwapLayout.span,
        programId: swapProgramId,
      }),
    );

    const instruction = TokenSwap.createInitSwapInstruction(
      tokenSwapAccount,
      authority,
      nonce,
      tokenAccountA,
      tokenAccountB,
      poolToken,
      tokenAccountPool,
      tokenProgramId,
      swapProgramId,
      feeNumerator,
      feeDenominator,
    );

    transaction.add(instruction);
    await sendAndConfirmTransaction(
      'createAccount and InitializeSwap',
      connection,
      transaction,
      payer,
      tokenSwapAccount,
    );

    return tokenSwap;
  }

  /**
   * Retrieve tokenSwap information
   */
  async getInfo(): Promise<TokenSwapInfo> {
    const tokenSwapInfo = await TokenSwap.loadTokenSwapInfo(
      this.connection,
      this.tokenSwap,
      this.swapProgramId,
    );

    // already properly filled in
    // tokenSwapInfo.nonce = tokenSwapInfo.nonce;
    tokenSwapInfo.tokenProgramId = new PublicKey(tokenSwapInfo.tokenProgramId);
    tokenSwapInfo.tokenAccountA = new PublicKey(tokenSwapInfo.tokenAccountA);
    tokenSwapInfo.tokenAccountB = new PublicKey(tokenSwapInfo.tokenAccountB);
    tokenSwapInfo.tokenPool = new PublicKey(tokenSwapInfo.tokenPool);
    tokenSwapInfo.feesNumerator = Numberu64.fromBuffer(
      tokenSwapInfo.feesNumerator,
    );
    tokenSwapInfo.feesDenominator = Numberu64.fromBuffer(
      tokenSwapInfo.feesDenominator,
    );
    tokenSwapInfo.feeRatio =
      tokenSwapInfo.feesNumerator.toNumber() /
      tokenSwapInfo.feesDenominator.toNumber();

    return tokenSwapInfo;
  }

  /**
   * Swap token A for token B
   *
   * @param source Source token account
   * @param destination Destination token account
   * @param amountIn Amount to transfer from source account
   * @param minimumAmountOut Minimum amount to send
   */
  async swap(
    source: PublicKey,
    destination: PublicKey,
    amountIn: number | Numberu64,
    minimumAmountOut: number | Numberu64,
  ): Promise<TransactionSignature> {
    return await sendAndConfirmTransaction(
      'swap',
      this.connection,
      new Transaction().add(
        TokenSwap.swapInstruction(
          this.tokenSwap,
          this.authority,
          source,
          this.tokenAccountA,
          this.tokenAccountB,
          destination,
          this.swapProgramId,
          this.tokenProgramId,
          amountIn,
          minimumAmountOut,
        ),
      ),
      this.payer,
    );
  }

  /**
   * Swap token B for token A
   *
   * @param source Source token account
   * @param destination Destination token account
   * @param amountIn Amount to transfer from source account
   * @param minimumAmountOut Minimum amount to send
   */
  async swapInverse(
    source: PublicKey,
    destination: PublicKey,
    amountIn: number | Numberu64,
    minimumAmountOut: number | Numberu64,
  ): Promise<TransactionSignature> {
    return await sendAndConfirmTransaction(
      'swap',
      this.connection,
      new Transaction().add(
        TokenSwap.swapInstruction(
          this.tokenSwap,
          this.authority,
          source,
          this.tokenAccountB,
          this.tokenAccountA,
          destination,
          this.swapProgramId,
          this.tokenProgramId,
          amountIn,
          minimumAmountOut,
        ),
      ),
      this.payer,
    );
  }

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
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('amountIn'),
      Layout.uint64('minimumAmountOut'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 1, // Swap instruction
        amountIn: new Numberu64(amountIn).toBuffer(),
        minimumAmountOut: new Numberu64(minimumAmountOut).toBuffer(),
      },
      data,
    );

    const keys = [
      {pubkey: tokenSwap, isSigner: false, isWritable: false},
      {pubkey: authority, isSigner: false, isWritable: false},
      {pubkey: source, isSigner: false, isWritable: true},
      {pubkey: swapSource, isSigner: false, isWritable: true},
      {pubkey: swapDestination, isSigner: false, isWritable: true},
      {pubkey: destination, isSigner: false, isWritable: true},
      {pubkey: tokenProgramId, isSigner: false, isWritable: false},
    ];
    return new TransactionInstruction({
      keys,
      programId: swapProgramId,
      data,
    });
  }

  /**
   * Deposit tokens into the pool
   * @param userAccountA User account for token A
   * @param userAccountB User account for token B
   * @param poolAccount User account for pool token
   * @param poolTokenAmount Amount of pool tokens to mint
   * @param maximumTokenA The maximum amount of token A to deposit
   * @param maximumTokenB The maximum amount of token B to deposit
   */
  async deposit(
    userAccountA: PublicKey,
    userAccountB: PublicKey,
    poolAccount: PublicKey,
    poolTokenAmount: number | Numberu64,
    maximumTokenA: number | Numberu64,
    maximumTokenB: number | Numberu64,
  ): Promise<TransactionSignature> {
    return await sendAndConfirmTransaction(
      'deposit',
      this.connection,
      new Transaction().add(
        TokenSwap.depositInstruction(
          this.tokenSwap,
          this.authority,
          userAccountA,
          userAccountB,
          this.tokenAccountA,
          this.tokenAccountB,
          this.poolToken,
          poolAccount,
          this.swapProgramId,
          this.tokenProgramId,
          poolTokenAmount,
          maximumTokenA,
          maximumTokenB,
        ),
      ),
      this.payer,
    );
  }

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
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('poolTokenAmount'),
      Layout.uint64('maximumTokenA'),
      Layout.uint64('maximumTokenB'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 2, // Deposit instruction
        poolTokenAmount: new Numberu64(poolTokenAmount).toBuffer(),
        maximumTokenA: new Numberu64(maximumTokenA).toBuffer(),
        maximumTokenB: new Numberu64(maximumTokenB).toBuffer(),
      },
      data,
    );

    const keys = [
      {pubkey: tokenSwap, isSigner: false, isWritable: false},
      {pubkey: authority, isSigner: false, isWritable: false},
      {pubkey: sourceA, isSigner: false, isWritable: true},
      {pubkey: sourceB, isSigner: false, isWritable: true},
      {pubkey: intoA, isSigner: false, isWritable: true},
      {pubkey: intoB, isSigner: false, isWritable: true},
      {pubkey: poolToken, isSigner: false, isWritable: true},
      {pubkey: poolAccount, isSigner: false, isWritable: true},
      {pubkey: tokenProgramId, isSigner: false, isWritable: false},
    ];
    return new TransactionInstruction({
      keys,
      programId: swapProgramId,
      data,
    });
  }

  /**
   * Withdraw tokens from the pool
   *
   * @param userAccountA User account for token A
   * @param userAccountB User account for token B
   * @param poolAccount User account for pool token
   * @param poolTokenAmount Amount of pool tokens to burn
   * @param minimumTokenA The minimum amount of token A to withdraw
   * @param minimumTokenB The minimum amount of token B to withdraw
   */
  async withdraw(
    userAccountA: PublicKey,
    userAccountB: PublicKey,
    poolAccount: PublicKey,
    poolTokenAmount: number | Numberu64,
    minimumTokenA: number | Numberu64,
    minimumTokenB: number | Numberu64,
  ): Promise<TransactionSignature> {
    return await sendAndConfirmTransaction(
      'withdraw',
      this.connection,
      new Transaction().add(
        TokenSwap.withdrawInstruction(
          this.tokenSwap,
          this.authority,
          this.poolToken,
          poolAccount,
          this.tokenAccountA,
          this.tokenAccountB,
          userAccountA,
          userAccountB,
          this.swapProgramId,
          this.tokenProgramId,
          poolTokenAmount,
          minimumTokenA,
          minimumTokenB,
        ),
      ),
      this.payer,
    );
  }

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
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('poolTokenAmount'),
      Layout.uint64('minimumTokenA'),
      Layout.uint64('minimumTokenB'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 3, // Withdraw instruction
        poolTokenAmount: new Numberu64(poolTokenAmount).toBuffer(),
        minimumTokenA: new Numberu64(minimumTokenA).toBuffer(),
        minimumTokenB: new Numberu64(minimumTokenB).toBuffer(),
      },
      data,
    );

    const keys = [
      {pubkey: tokenSwap, isSigner: false, isWritable: false},
      {pubkey: authority, isSigner: false, isWritable: false},
      {pubkey: poolMint, isSigner: false, isWritable: true},
      {pubkey: sourcePoolAccount, isSigner: false, isWritable: true},
      {pubkey: fromA, isSigner: false, isWritable: true},
      {pubkey: fromB, isSigner: false, isWritable: true},
      {pubkey: userAccountA, isSigner: false, isWritable: true},
      {pubkey: userAccountB, isSigner: false, isWritable: true},
      {pubkey: tokenProgramId, isSigner: false, isWritable: false},
    ];
    return new TransactionInstruction({
      keys,
      programId: swapProgramId,
      data,
    });
  }
}
