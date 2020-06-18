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

import {newAccountWithLamports} from '../client/util/new-account-with-lamports';
import * as Layout from './layout';
import {sendAndConfirmTransaction} from './util/send-and-confirm-transaction';

/**
 * Some amount of tokens
 */
export class TokenAmount extends BN {
  /**
   * Convert to Buffer representation
   */
  toBuffer(): Buffer {
    const a = super.toArray().reverse();
    const b = Buffer.from(a);
    if (b.length === 8) {
      return b;
    }
    assert(b.length < 8, 'TokenAmount too large');

    const zeroPad = Buffer.alloc(8);
    b.copy(zeroPad);
    return zeroPad;
  }

  /**
   * Construct a TokenAmount from Buffer representation
   */
  static fromBuffer(buffer: Buffer): TokenAmount {
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
 * Information about a token
 */
type TokenInfo = {|
  /**
   * Total supply of tokens
   */
  supply: TokenAmount,

  /**
   * Number of base 10 digits to the right of the decimal place
   */
  decimals: number,
  /**
   * Owner of the token, given authority to mint new tokens
   */
  owner: null | PublicKey,
|};

/**
 * @private
 */
const TokenInfoLayout = BufferLayout.struct([
  BufferLayout.u8('state'),
  Layout.uint64('supply'),
  BufferLayout.u8('decimals'),
]);

const tokenLayout = BufferLayout.struct([
  BufferLayout.u8('state'),
  Layout.uint64('supply'),
  BufferLayout.nu64('decimals'),
  BufferLayout.u8('option'),
  Layout.publicKey('owner'),
  BufferLayout.nu64('padding'),
]);

/**
 * Information about an account
 */
type AccountInfo = {|
  /**
   * The kind of token this account holds
   */
  token: PublicKey,

  /**
   * Owner of this account
   */
  owner: PublicKey,

  /**
   * Amount of tokens this account holds
   */
  amount: TokenAmount,

  /**
   * The source account for the tokens.
   *
   * If `source` is null, the source is this account.
   * If `source` is not null, the `amount` of tokens in this account represent
   * an allowance of tokens that may be transferred from the source account
   */
  source: null | PublicKey,

  /**
   * Original amount of tokens this delegate account was authorized to spend
   * If `source` is null, originalAmount is zero
   */
  originalAmount: TokenAmount,
|};

/**
 * @private
 */
const AccountInfoLayout = BufferLayout.struct([
  BufferLayout.u8('state'),
  Layout.publicKey('token'),
  Layout.publicKey('owner'),
  Layout.uint64('amount'),
  BufferLayout.nu64('option'),
  Layout.publicKey('source'),
  Layout.uint64('originalAmount'),
]);

type TokenAndPublicKey = [Token, PublicKey]; // This type exists to workaround an esdoc parse error

/**
 * An ERC20-like Token
 */
export class Token {
  /**
   * @private
   */
  connection: Connection;

  /**
   * The public key identifying this token
   */
  token: PublicKey;

  /**
   * Program Identifier for the Token program
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
  constructor(connection: Connection, token: PublicKey, programId: PublicKey, payer: Account) {
    Object.assign(this, {connection, token, programId, payer});
  }

  /**
   * Get the minimum balance for the token to be rent exempt
   *
   * @return Number of lamports required
   */
  static async getMinBalanceRentForExemptToken(
    connection: Connection,
  ): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(
      TokenInfoLayout.span,
    );
  }

  /**
   * Get the minimum balance for the account to be rent exempt
   *
   * @return Number of lamports required
   */
  static async getMinBalanceRentForExemptAccount(
    connection: Connection,
  ): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(
      AccountInfoLayout.span,
    );
  }

  /**
   * Create a new Token
   *
   * @param connection The connection to use
   * @param owner User account that will own the returned account
   * @param supply Total supply of the new token
   * @param decimals Location of the decimal place
   * @param programId Optional token programId, uses the system programId by default
   * @return Token object for the newly minted token, Public key of the account holding the total supply of new tokens
   */
  static async createNewToken(
    connection: Connection,
    owner: Account,
    supply: TokenAmount,
    decimals: number,
    programId: PublicKey,
    is_owned: boolean = false,
  ): Promise<TokenAndPublicKey> {
    const tokenAccount = new Account();
    const payer = await newAccountWithLamports(connection, 10000000 /* wag */);
    const token = new Token(connection, tokenAccount.publicKey, programId, payer);
    const initialAccountPublicKey = await token.newAccount(owner, null);

    let transaction;

    const commandDataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('supply'),
      BufferLayout.nu64('decimals'),
    ]);

    let data = Buffer.alloc(1024);
    {
      const encodeLength = commandDataLayout.encode(
        {
          instruction: 0, // NewToken instruction
          supply: supply.toBuffer(),
          decimals,
        },
        data,
      );
      data = data.slice(0, encodeLength);
    }

    const balanceNeeded = await Token.getMinBalanceRentForExemptToken(
      connection,
    );

    // Allocate memory for the account
    transaction = SystemProgram.createAccount({
      fromPubkey: owner.publicKey,
      newAccountPubkey: tokenAccount.publicKey,
      lamports: balanceNeeded,
      space: tokenLayout.span,
      programId,
    });
    await sendAndConfirmTransaction(
      'createAccount',
      connection,
      transaction,
      payer,
      owner,
      tokenAccount,
    );

    let keys = [
      {pubkey: tokenAccount.publicKey, isSigner: true, isWritable: false},
      {pubkey: initialAccountPublicKey, isSigner: false, isWritable: true},
    ];
    if (is_owned) {
      keys.push({pubkey: owner.publicKey, isSigner: true, isWritable: false});
    }

    transaction = new Transaction().add({
      keys,
      programId,
      data,
    });
    await sendAndConfirmTransaction(
      'New Account',
      connection,
      transaction,
      payer,
      owner,
      tokenAccount,
    );

    return [token, initialAccountPublicKey];
  }

  /**
   * Create a new and empty account.
   *
   * This account may then be used as a `transfer()` or `approve()` destination
   *
   * @param owner User account that will own the new account
   * @param source If not null, create a delegate account that when authorized
   *               may transfer tokens from this `source` account
   * @return Public key of the new empty account
   */
  async newAccount(
    owner: Account,
    source: null | PublicKey = null,
  ): Promise<PublicKey> {
    const tokenAccount = new Account();
    let transaction;

    const dataLayout = BufferLayout.struct([BufferLayout.u8('instruction')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 1, // NewAccount instruction
      },
      data,
    );

    const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(
      this.connection,
    );

    // Allocate memory for the token
    transaction = SystemProgram.createAccount({
      fromPubkey: owner.publicKey,
      newAccountPubkey: tokenAccount.publicKey,
      lamports: balanceNeeded,
      space: AccountInfoLayout.span,
      programId: this.programId,
    });
    await sendAndConfirmTransaction(
      'createAccount',
      this.connection,
      transaction,
      this.payer,
      owner,
      tokenAccount,
    );

    // Initialize the account
    const keys = [
      {pubkey: tokenAccount.publicKey, isSigner: true, isWritable: true},
      {pubkey: owner.publicKey, isSigner: false, isWritable: false},
      {pubkey: this.token, isSigner: false, isWritable: false},
    ];
    if (source) {
      keys.push({pubkey: source, isSigner: false, isWritable: false});
    }
    transaction = new Transaction().add({
      keys,
      programId: this.programId,
      data,
    });
    await sendAndConfirmTransaction(
      'init account',
      this.connection,
      transaction,
      this.payer,
      owner,
      tokenAccount,
    );

    return tokenAccount.publicKey;
  }

  /**
   * Retrieve token information
   */
  async tokenInfo(): Promise<TokenInfo> {
    const accountInfo = await this.connection.getAccountInfo(this.token);
    if (accountInfo === null) {
      throw new Error('Failed to find token info account');
    }
    if (!accountInfo.owner.equals(this.programId)) {
      throw new Error(
        `Invalid token owner: ${JSON.stringify(accountInfo.owner)}`,
      );
    }

    const data = Buffer.from(accountInfo.data);

    const tokenInfo = tokenLayout.decode(data);
    if (tokenInfo.state !== 1) {
      throw new Error(`Invalid account data`);
    }
    tokenInfo.supply = TokenAmount.fromBuffer(tokenInfo.supply);
    if (tokenInfo.option === 0) {
      tokenInfo.owner = null;
    } else {
      tokenInfo.owner = new PublicKey(tokenInfo.owner);
    }
    return tokenInfo;
  }

  /**
   * Retrieve account information
   *
   * @param account Public key of the account
   */
  async accountInfo(account: PublicKey): Promise<AccountInfo> {
    const accountInfo = await this.connection.getAccountInfo(account);
    if (accountInfo === null) {
      throw new Error('Failed to find account');
    }
    if (!accountInfo.owner.equals(this.programId)) {
      throw new Error(`Invalid account owner`);
    }

    const data = Buffer.from(accountInfo.data);
    const tokenAccountInfo = AccountInfoLayout.decode(data);

    if (tokenAccountInfo.state !== 2) {
      throw new Error(`Invalid account data`);
    }
    tokenAccountInfo.token = new PublicKey(tokenAccountInfo.token);
    tokenAccountInfo.owner = new PublicKey(tokenAccountInfo.owner);
    tokenAccountInfo.amount = TokenAmount.fromBuffer(tokenAccountInfo.amount);
    if (tokenAccountInfo.option === 0) {
      tokenAccountInfo.source = null;
      tokenAccountInfo.originalAmount = new TokenAmount();
    } else {
      tokenAccountInfo.source = new PublicKey(tokenAccountInfo.source);
      tokenAccountInfo.originalAmount = TokenAmount.fromBuffer(
        tokenAccountInfo.originalAmount,
      );
    }

    if (!tokenAccountInfo.token.equals(this.token)) {
      throw new Error(
        `Invalid account token: ${JSON.stringify(
          tokenAccountInfo.token,
        )} !== ${JSON.stringify(this.token)}`,
      );
    }
    return tokenAccountInfo;
  }

  /**
   * Transfer tokens to another account
   *
   * @param owner Owner of the source account
   * @param source Source account
   * @param destination Destination account
   * @param amount Number of tokens to transfer
   */
  async transfer(
    owner: Account,
    source: PublicKey,
    destination: PublicKey,
    amount: number | TokenAmount,
  ): Promise<?TransactionSignature> {
    return await sendAndConfirmTransaction(
      'transfer',
      this.connection,
      new Transaction().add(
        await this.transferInstruction(
          owner.publicKey,
          source,
          destination,
          amount,
        ),
      ),
      this.payer,
      owner,
    );
  }

  /**
   * Grant a third-party permission to transfer up the specified number of tokens from an account
   *
   * @param owner Owner of the source account
   * @param account Public key of the account
   * @param delegate Account authorized to perform a transfer tokens from the source account
   * @param amount Maximum number of tokens the delegate may transfer
   */
  async approve(
    owner: Account,
    account: PublicKey,
    delegate: PublicKey,
    amount: number | TokenAmount,
  ): Promise<void> {
    await sendAndConfirmTransaction(
      'approve',
      this.connection,
      new Transaction().add(
        this.approveInstruction(owner.publicKey, account, delegate, amount),
      ),
      this.payer,
      owner,
    );
  }

  /**
   * Remove approval for the transfer of any remaining tokens
   *
   * @param owner Owner of the source account
   * @param account Public key of the account
   * @param delegate Account to revoke authorization from
   */
  revoke(
    owner: Account,
    account: PublicKey,
    delegate: PublicKey,
  ): Promise<void> {
    return this.approve(owner, account, delegate, 0);
  }

  /**
   * Assign a new owner to the account
   *
   * @param owner Owner of the account
   * @param account Public key of the account
   * @param newOwner New owner of the account
   */
  async setOwner(
    owner: Account,
    owned: PublicKey,
    newOwner: PublicKey,
  ): Promise<void> {
    await sendAndConfirmTransaction(
      'setOwneer',
      this.connection,
      new Transaction().add(
        this.setOwnerInstruction(owner.publicKey, owned, newOwner),
      ),
      this.payer,
      owner,
    );
  }

  /**
   * Mint new tokens
   *
   * @param token Public key of the token
   * @param owner Owner of the token
   * @param dest Public key of the account to mint to
   * @param amount ammount to mint
   */
  async mintTo(
    owner: Account,
    token: PublicKey,
    dest: PublicKey,
    amount: number,
  ): Promise<void> {
    await sendAndConfirmTransaction(
      'mintTo',
      this.connection,
      new Transaction().add(this.mintToInstruction(owner, token, dest, amount)),
      this.payer,
      owner,
    );
  }

  /**
   * Burn tokens
   *
   * @param owner Public key account owner
   * @param account Account to burn tokens from
   * @param amount ammount to burn
   */
  async burn(
    owner: Account,
    account: PublicKey,
    amount: number,
  ): Promise<void> {
    await sendAndConfirmTransaction(
      'burn',
      this.connection,
      new Transaction().add(await this.burnInstruction(owner, account, amount)),
      this.payer,
      owner,
    );
  }

  /**
   * Construct a Transfer instruction
   *
   * @param owner Owner of the source account
   * @param source Source account
   * @param destination Destination account
   * @param amount Number of tokens to transfer
   */
  async transferInstruction(
    owner: PublicKey,
    source: PublicKey,
    destination: PublicKey,
    amount: number | TokenAmount,
  ): Promise<TransactionInstruction> {
    const accountInfo = await this.accountInfo(source);
    if (!owner.equals(accountInfo.owner)) {
      throw new Error('Account owner mismatch');
    }

    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('amount'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 2, // Transfer instruction
        amount: new TokenAmount(amount).toBuffer(),
      },
      data,
    );

    const keys = [
      {pubkey: owner, isSigner: true, isWritable: false},
      {pubkey: source, isSigner: false, isWritable: true},
      {pubkey: destination, isSigner: false, isWritable: true},
    ];
    if (accountInfo.source) {
      keys.push({
        pubkey: accountInfo.source,
        isSigner: false,
        isWritable: true,
      });
    }
    return new TransactionInstruction({
      keys,
      programId: this.programId,
      data,
    });
  }

  /**
   * Construct an Approve instruction
   *
   * @param owner Owner of the source account
   * @param account Public key of the account
   * @param delegate Account authorized to perform a transfer tokens from the source account
   * @param amount Maximum number of tokens the delegate may transfer
   */
  approveInstruction(
    owner: PublicKey,
    account: PublicKey,
    delegate: PublicKey,
    amount: number | TokenAmount,
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('amount'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 3, // Approve instruction
        amount: new TokenAmount(amount).toBuffer(),
      },
      data,
    );

    return new TransactionInstruction({
      keys: [
        {pubkey: owner, isSigner: true, isWritable: false},
        {pubkey: account, isSigner: false, isWritable: false},
        {pubkey: delegate, isSigner: false, isWritable: true},
      ],
      programId: this.programId,
      data,
    });
  }

  /**
   * Construct an Revoke instruction
   *
   * @param owner Owner of the source account
   * @param account Public key of the account
   * @param delegate Account authorized to perform a transfer tokens from the source account
   */
  revokeInstruction(
    owner: PublicKey,
    account: PublicKey,
    delegate: PublicKey,
  ): TransactionInstruction {
    return this.approveInstruction(owner, account, delegate, 0);
  }

  /**
   * Construct a SetOwner instruction
   *
   * @param owner Owner of the account
   * @param account Public key of the account
   * @param newOwner New owner of the account
   */
  setOwnerInstruction(
    owner: PublicKey,
    owned: PublicKey,
    newOwner: PublicKey,
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([BufferLayout.u8('instruction')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 4, // SetOwner instruction
      },
      data,
    );

    return new TransactionInstruction({
      keys: [
        {pubkey: owner, isSigner: true, isWritable: false},
        {pubkey: owned, isSigner: false, isWritable: true},
        {pubkey: newOwner, isSigner: false, isWritable: false},
      ],
      programId: this.programId,
      data,
    });
  }

  /**
   * Construct a MintTo instruction
   *
   * @param token Public key of the token
   * @param owner Owner of the token
   * @param dest Public key of the account to mint to
   * @param amount ammount to mint
   */
  mintToInstruction(
    owner: Account,
    token: PublicKey,
    dest: PublicKey,
    amount: number,
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('amount'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 5, // MintTo instruction
        amount: new TokenAmount(amount).toBuffer(),
      },
      data,
    );

    return new TransactionInstruction({
      keys: [
        {pubkey: owner.publicKey, isSigner: true, isWritable: false},
        {pubkey: token, isSigner: false, isWritable: true},
        {pubkey: dest, isSigner: false, isWritable: true},
      ],
      programId: this.programId,
      data,
    });
  }

  /**
   * Construct a Burn instruction
   *
   * @param owner Public key account owner
   * @param account Account to burn tokens from
   * @param amount ammount to burn
   */
  async burnInstruction(
    owner: Account,
    account: PublicKey,
    amount: number,
  ): Promise<TransactionInstruction> {
    const accountInfo = await this.accountInfo(account);

    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('amount'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 6, // Burn instruction
        amount: new TokenAmount(amount).toBuffer(),
      },
      data,
    );

    const keys = [
      {pubkey: owner.publicKey, isSigner: true, isWritable: false},
      {pubkey: account, isSigner: false, isWritable: true},
      {pubkey: this.token, isSigner: false, isWritable: true},
    ];
    if (accountInfo.source) {
      keys.push({
        pubkey: accountInfo.source,
        isSigner: false,
        isWritable: true,
      });
    }

    return new TransactionInstruction({
      keys,
      programId: this.programId,
      data,
    });
  }
}
