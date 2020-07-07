/**
 * @flow
 */

import assert from 'assert';
import BN from 'bn.js';
import * as BufferLayout from 'buffer-layout';
import {
  Account,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
} from '@solana/web3.js';
import type {Connection, TransactionSignature} from '@solana/web3.js';

import * as Layout from './layout';
import {sendAndConfirmTransaction} from './util/send-and-confirm-transaction';

/**
 * Some amount of tokens
 */
export class Numberu64 extends BN {
  /**
   * Convert to Buffer representation
   */
  toBuffer(): Buffer {
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
  static fromBuffer(buffer: Buffer): Numberu64 {
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
const TokenSwapLayout = BufferLayout.struct([
  BufferLayout.u8('state'),
  Layout.publicKey('tokenAccountA'),
  Layout.publicKey('tokenAccountB'),
  Layout.publicKey('tokenPool'),
  Layout.uint64('feesDenominator'),
  Layout.uint64('feesNumerator'),
]);

/**
 * An ERC20-like Token
 */
export class TokenSwap {
  /**
   * @private
   */
  connection: Connection;

  /**
   * The public key identifying this token
   */
  tokenSwap: PublicKey;

  /**
   * Program Identifier for the Token Swap program
   */
  programId: PublicKey;

  /**
   * Fee payer
   */
  payer: Account;

  /**
   * Create a Token object attached to the specific token
   *
   * @param connection The connection to use
   * @param token Public key of the token
   * @param programId Optional token programId, uses the system programId by default
   * @param payer Payer of fees
   */
  constructor(connection: Connection, tokenSwap: PublicKey, programId: PublicKey, payer: Account) {
    Object.assign(this, {connection, tokenSwap, programId, payer});
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

  /**
   * Create a new Token Swap
   *
   * @param connection The connection to use
   * @param payer Pays for the transaction
   * @param tokenSwapAccount The token swap account
   * @param authority The authority over the swap and accounts
   * @param tokenAccountA: The Swap's Token A account
   * @param tokenAccountB: The Swap's Token B account
   * @param tokenPool The pool token
   * @param tokenAccountPool The pool token account
   * @param tokenProgramId The program id of the token program
   * @param feeNumerator Numerator of the fee ratio
   * @param feeDenominator Denominator of the fee ratio
   * @param programId Program ID of the token-swap program
   * @return Token object for the newly minted token, Public key of the account holding the total supply of new tokens
   */
  static async createTokenSwap(
    connection: Connection,
    payer: Account,
    tokenSwapAccount: Account,
    authority: PublicKey,
    tokenAccountA: PublicKey,
    tokenAccountB: PublicKey,
    tokenPool: PublicKey,
    tokenAccountPool: PublicKey,
    tokenProgramId: PublicKey,
    feeNumerator: number,
    feeDenominator: number,
    programId: PublicKey,
  ): Promise<TokenSwap> {
    let transaction;
    const tokenSwap = new TokenSwap(connection, tokenSwapAccount.publicKey, programId, payer);

    // Allocate memory for the account
    const balanceNeeded = await TokenSwap.getMinBalanceRentForExemptTokenSwap(
      connection,
    );
    transaction = SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: tokenSwapAccount.publicKey,
      lamports: balanceNeeded,
      space: TokenSwapLayout.span,
      programId,
    });
    await sendAndConfirmTransaction(
      'createAccount',
      connection,
      transaction,
      payer,
      tokenSwapAccount,
    );

    let keys = [
      {pubkey: tokenSwapAccount.publicKey, isSigner: true, isWritable: true},
      {pubkey: authority, isSigner: false, isWritable: false},
      {pubkey: tokenAccountA, isSigner: false, isWritable: true},
      {pubkey: tokenAccountB, isSigner: false, isWritable: true},
      {pubkey: tokenPool, isSigner: false, isWritable: true},
      {pubkey: tokenAccountPool, isSigner: false, isWritable: true},
      {pubkey: tokenProgramId, isSigner: false, isWritable: false},
    ];
    const commandDataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      BufferLayout.nu64('feeDenominator'),
      BufferLayout.nu64('feeNumerator'),
    ]);
    let data = Buffer.alloc(1024);
    {
      const encodeLength = commandDataLayout.encode(
        {
          instruction: 0, // InitializeSwap instruction
          feeNumerator,
          feeDenominator,
        },
        data,
      );
      data = data.slice(0, encodeLength);
    }
    transaction = new Transaction().add({
      keys,
      programId,
      data,
    });
    await sendAndConfirmTransaction(
      'InitializeSwap',
      connection,
      transaction,
      payer,
      tokenSwapAccount
    );

    return tokenSwap;
  }

  /**
   * Retrieve tokenSwap information
   */
  async getInfo(): Promise<TokenSwapInfo> {
    const accountInfo = await this.connection.getAccountInfo(this.tokenSwap);
    if (accountInfo === null) {
      throw new Error('Failed to find token swap account');
    }
    if (!accountInfo.owner.equals(this.programId)) {
      throw new Error(
        `Invalid token swap owner: ${JSON.stringify(accountInfo.owner)}`,
      );
    }

    const data = Buffer.from(accountInfo.data);
    const tokenSwapInfo = TokenSwapLayout.decode(data);
    if (tokenSwapInfo.state !== 1) {
      throw new Error(`Invalid token swap state`);
    }
    tokenSwapInfo.tokenAccountA = new PublicKey(tokenSwapInfo.tokenAccountA);
    tokenSwapInfo.tokenAccountB = new PublicKey(tokenSwapInfo.tokenAccountB);
    tokenSwapInfo.tokenPool = new PublicKey(tokenSwapInfo.tokenPool);
    tokenSwapInfo.feesNumerator = Numberu64.fromBuffer(tokenSwapInfo.feesNumerator);
    tokenSwapInfo.feesDenominator = Numberu64.fromBuffer(tokenSwapInfo.feesDenominator);
    tokenSwapInfo.feeRatio = tokenSwapInfo.feesNumerator.toNumber() / tokenSwapInfo.feesDenominator.toNumber();

    return tokenSwapInfo;
  }

  /**
   * Swap the tokens in the pool
   *
   * @param authority Authority
   * @param source Source account
   * @param into Base account to swap into, must be a source token
   * @param from Base account to swap from, must be a destination token
   * @param dest Destination token
   * @param tokenProgramId Token program id
   * @param amount Amount to transfer from source account
   */
  async swap(
    authority: PublicKey,
    source: PublicKey,
    into: PublicKey,
    from: PublicKey,
    destination: PublicKey,
    tokenProgramId: PublicKey,
    amount: number | Numberu64,
  ): Promise<?TransactionSignature> {
    return await sendAndConfirmTransaction(
      'swap',
      this.connection,
      new Transaction().add(
        this.swapInstruction(
          authority,
          source,
          into,
          from,
          destination,
          tokenProgramId,
          amount,
        ),
      ),
      this.payer,
    );
  }
  swapInstruction(
    authority: PublicKey,
    source: PublicKey,
    into: PublicKey,
    from: PublicKey,
    destination: PublicKey,
    tokenProgramId: PublicKey,
    amount: number | Numberu64,
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('amount'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 1, // Swap instruction
        amount: new Numberu64(amount).toBuffer(),
      },
      data,
    );

    const keys = [
      {pubkey: this.tokenSwap, isSigner: false, isWritable: false},
      {pubkey: authority, isSigner: false, isWritable: false},
      {pubkey: source, isSigner: false, isWritable: true},
      {pubkey: into, isSigner: false, isWritable: true},
      {pubkey: from, isSigner: false, isWritable: true},
      {pubkey: destination, isSigner: false, isWritable: true},
      {pubkey: tokenProgramId, isSigner: false, isWritable: false},

    ];
    return new TransactionInstruction({
      keys,
      programId: this.programId,
      data,
    });
  }

  /**
   * Deposit some tokens into the pool
   *
   * @param authority Authority
   * @param sourceA Source account A
   * @param sourceB Source account B
   * @param intoA Base account A to deposit into
   * @param intoB Base account B to deposit into
   * @param poolToken Pool token
   * @param poolAccount Pool account to deposit the generated tokens
   * @param tokenProgramId Token program id
   * @param amount Amount of token A to transfer, token B amount is set by the exchange rate
   */
  async deposit(
    authority: PublicKey,
    sourceA: PublicKey,
    sourceB: PublicKey,
    intoA: PublicKey,
    intoB: PublicKey,
    poolToken: PublicKey,
    poolAccount: PublicKey,
    tokenProgramId: PublicKey,
    amount: number | Numberu64,
  ): Promise<?TransactionSignature> {
    return await sendAndConfirmTransaction(
      'deposit',
      this.connection,
      new Transaction().add(
        this.depositInstruction(
          authority,
          sourceA,
          sourceB,
          intoA,
          intoB,
          poolToken,
          poolAccount,
          tokenProgramId,
          amount,
        ),
      ),
      this.payer,
    );
  }
  depositInstruction(
    authority: PublicKey,
    sourceA: PublicKey,
    sourceB: PublicKey,
    intoA: PublicKey,
    intoB: PublicKey,
    poolToken: PublicKey,
    poolAccount: PublicKey,
    tokenProgramId: PublicKey,
    amount: number | Numberu64,
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('amount'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 2, // Deposit instruction
        amount: new Numberu64(amount).toBuffer(),
      },
      data,
    );

    const keys = [
      {pubkey: this.tokenSwap, isSigner: false, isWritable: false},
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
      programId: this.programId,
      data,
    });
  }

  /**
   * Withdraw the token from the pool at the current ratio
   *
   * @param authority Authority
   * @param sourcePoolAccount Source pool account
   * @param poolToken Pool token
   * @param fromA Base account A to withdraw from
   * @param fromB Base account B to withdraw from
   * @param userAccountA Token A user account
   * @param userAccountB token B user account
   * @param tokenProgramId Token program id
   * @param amount Amount of token A to transfer, token B amount is set by the exchange rate
   */
  async withdraw(
    authority: PublicKey,
    sourcePoolAccount: PublicKey,
    fromA: PublicKey,
    fromB: PublicKey,
    userAccountA: PublicKey,
    userAccountB: PublicKey,
    tokenProgramId: PublicKey,
    amount: number | Numberu64,
  ): Promise<?TransactionSignature> {
    return await sendAndConfirmTransaction(
      'withdraw',
      this.connection,
      new Transaction().add(
        this.withdrawInstruction(
          authority,
          sourcePoolAccount,
          fromA,
          fromB,
          userAccountA,
          userAccountB,
          tokenProgramId,
          amount,
        ),
      ),
      this.payer,
    );
  }
  withdrawInstruction(
    authority: PublicKey,
    sourcePoolAccount: PublicKey,
    fromA: PublicKey,
    fromB: PublicKey,
    userAccountA: PublicKey,
    userAccountB: PublicKey,
    tokenProgramId: PublicKey,
    amount: number | Numberu64,
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('amount'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 3, // Withdraw instruction
        amount: new Numberu64(amount).toBuffer(),
      },
      data,
    );

    const keys = [
      {pubkey: this.tokenSwap, isSigner: false, isWritable: false},
      {pubkey: authority, isSigner: false, isWritable: false},
      {pubkey: sourcePoolAccount, isSigner: false, isWritable: true},
      {pubkey: fromA, isSigner: false, isWritable: true},
      {pubkey: fromB, isSigner: false, isWritable: true},
      {pubkey: userAccountA, isSigner: false, isWritable: true},
      {pubkey: userAccountB, isSigner: false, isWritable: true},
      {pubkey: tokenProgramId, isSigner: false, isWritable: false},

    ];
    return new TransactionInstruction({
      keys,
      programId: this.programId,
      data,
    });
  }
}
