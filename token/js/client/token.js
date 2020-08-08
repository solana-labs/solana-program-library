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
 * 64-bit value
 */
export class u64 extends BN {
  /**
   * Convert to Buffer representation
   */
  toBuffer(): Buffer {
    const a = super.toArray().reverse();
    const b = Buffer.from(a);
    if (b.length === 8) {
      return b;
    }
    assert(b.length < 8, 'u64 too large');

    const zeroPad = Buffer.alloc(8);
    b.copy(zeroPad);
    return zeroPad;
  }

  /**
   * Construct a u64 from Buffer representation
   */
  static fromBuffer(buffer: Buffer): u64 {
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

function isAccount(accountOrPublicKey: any): boolean {
  return 'publicKey' in accountOrPublicKey;
}

/**
 * Information about the mint
 */
type MintInfo = {|
  /**
   * Owner of the mint, given authority to mint new tokens
   */
  owner: null | PublicKey,

  /**
   * Number of base 10 digits to the right of the decimal place
   */
  decimals: number,

  /**
   * Is this mint initialized
   */
  initialized: boolean,
|};

const MintLayout = BufferLayout.struct([
  BufferLayout.u32('option'),
  Layout.publicKey('owner'),
  BufferLayout.u8('decimals'),
  BufferLayout.u8('is_initialized'),
  BufferLayout.u16('padding'),
]);

/**
 * Information about an account
 */
type AccountInfo = {|
  /**
   * The mint associated with this account
   */
  mint: PublicKey,

  /**
   * Owner of this account
   */
  owner: PublicKey,

  /**
   * Amount of tokens this account holds
   */
  amount: u64,

  /**
   * The delegate for this account
   */
  delegate: null | PublicKey,

  /**
   * The amount of tokens the delegate authorized to the delegate
   */
  delegatedAmount: u64,

  /**
   * Is this account initialized
   */
  isInitialized: boolean,

  /**
   * Is this a native token account
   */
  isNative: boolean,
|};

/**
 * @private
 */
const AccountLayout = BufferLayout.struct([
  Layout.publicKey('mint'),
  Layout.publicKey('owner'),
  Layout.uint64('amount'),
  BufferLayout.u32('option'),
  Layout.publicKey('delegate'),
  BufferLayout.u8('is_initialized'),
  BufferLayout.u8('is_native'),
  BufferLayout.u16('padding'),
  Layout.uint64('delegatedAmount'),
]);

/**
 * Information about an multisig
 */
type MultisigInfo = {|
  /**
   * The number of signers required
   */
  m: number,

  /**
   * Number of possible signers, corresponds to the
   * number of `signers` that are valid.
   */
  n: number,

  /**
   * Is this mint initialized
   */
  initialized: boolean,

  /**
   * The signers
   */
  signer1: PublicKey,
  signer2: PublicKey,
  signer3: PublicKey,
  signer4: PublicKey,
  signer5: PublicKey,
  signer6: PublicKey,
  signer7: PublicKey,
  signer8: PublicKey,
  signer9: PublicKey,
  signer10: PublicKey,
  signer11: PublicKey,
|};

/**
 * @private
 */
const MultisigLayout = BufferLayout.struct([
  BufferLayout.u8('m'),
  BufferLayout.u8('n'),
  BufferLayout.u8('is_initialized'),
  Layout.publicKey('signer1'),
  Layout.publicKey('signer2'),
  Layout.publicKey('signer3'),
  Layout.publicKey('signer4'),
  Layout.publicKey('signer5'),
  Layout.publicKey('signer6'),
  Layout.publicKey('signer7'),
  Layout.publicKey('signer8'),
  Layout.publicKey('signer9'),
  Layout.publicKey('signer10'),
  Layout.publicKey('signer11'),
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
   * The public key identifying this mint
   */
  publicKey: PublicKey;

  /**
   * Program Identifier for the Token program
   */
  programId: PublicKey;

  /**
   * Fee payer
   */
  payer: Account;

  /**
   * Create a Token object attached to the specific mint
   *
   * @param connection The connection to use
   * @param token Public key of the mint
   * @param programId token programId
   * @param payer Payer of fees
   */
  constructor(
    connection: Connection,
    publicKey: PublicKey,
    programId: PublicKey,
    payer: Account,
  ) {
    Object.assign(this, {connection, publicKey, programId, payer});
  }

  /**
   * Get the minimum balance for the mint to be rent exempt
   *
   * @return Number of lamports required
   */
  static async getMinBalanceRentForExemptMint(
    connection: Connection,
  ): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(MintLayout.span);
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
      AccountLayout.span,
    );
  }

  /**
   * Get the minimum balance for the multsig to be rent exempt
   *
   * @return Number of lamports required
   */
  static async getMinBalanceRentForExemptMultisig(
    connection: Connection,
  ): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(
      MultisigLayout.span,
    );
  }

  /**
   * Creates and initializes a token.
   *
   * @param connection The connection to use
   * @param owner User account that will own the returned account
   * @param supply Initial supply to mint
   * @param decimals Location of the decimal place
   * @param programId Optional token programId, uses the system programId by default
   * @return Token object for the newly minted token, Public key of the account
   *         holding the total amount of new tokens
   */
  static async createMint(
    connection: Connection,
    payer: Account,
    mintOwner: PublicKey,
    accountOwner: PublicKey,
    supply: u64,
    decimals: number,
    programId: PublicKey,
    is_owned: boolean = false,
  ): Promise<TokenAndPublicKey> {
    let transaction;
    const mintAccount = new Account();
    const token = new Token(
      connection,
      mintAccount.publicKey,
      programId,
      payer,
    );
    const initialAccountPublicKey = await token.createAccount(accountOwner);

    // Allocate memory for the account
    const balanceNeeded = await Token.getMinBalanceRentForExemptMint(
      connection,
    );
    transaction = SystemProgram.createAccount({
      fromPubkey: payer.publicKey,
      newAccountPubkey: mintAccount.publicKey,
      lamports: balanceNeeded,
      space: MintLayout.span,
      programId,
    });

    // Create the mint
    let keys = [
      {pubkey: mintAccount.publicKey, isSigner: false, isWritable: true},
    ];
    if (supply.toNumber() != 0) {
      keys.push({
        pubkey: initialAccountPublicKey,
        isSigner: false,
        isWritable: true,
      });
    }
    if (is_owned) {
      keys.push({pubkey: mintOwner, isSigner: false, isWritable: false});
    }
    const commandDataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('supply'),
      BufferLayout.u8('decimals'),
    ]);
    let data = Buffer.alloc(1024);
    {
      const encodeLength = commandDataLayout.encode(
        {
          instruction: 0, // InitializeMint instruction
          supply: supply.toBuffer(),
          decimals,
        },
        data,
      );
      data = data.slice(0, encodeLength);
    }
    transaction.add({
      keys,
      programId,
      data,
    });

    // Send the two instructions
    await sendAndConfirmTransaction(
      'createAccount and InitializeMint',
      connection,
      transaction,
      payer,
      mintAccount,
    );

    return [token, initialAccountPublicKey];
  }

  /**
   * Create and initializes a new account.
   *
   * This account may then be used as a `transfer()` or `approve()` destination
   *
   * @param owner User account that will own the new account
   * @return Public key of the new empty account
   */
  async createAccount(owner: PublicKey): Promise<PublicKey> {
    const mintAccount = new Account();
    let transaction;

    // Allocate memory for the account
    const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(
      this.connection,
    );
    transaction = SystemProgram.createAccount({
      fromPubkey: this.payer.publicKey,
      newAccountPubkey: mintAccount.publicKey,
      lamports: balanceNeeded,
      space: AccountLayout.span,
      programId: this.programId,
    });

    // create the new account
    const keys = [
      {pubkey: mintAccount.publicKey, isSigner: false, isWritable: true},
      {pubkey: this.publicKey, isSigner: false, isWritable: false},
      {pubkey: owner, isSigner: false, isWritable: false},
    ];
    const dataLayout = BufferLayout.struct([BufferLayout.u8('instruction')]);
    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 1, // InitializeAccount instruction
      },
      data,
    );
    transaction.add({
      keys,
      programId: this.programId,
      data,
    });

    // Send the two instructions
    await sendAndConfirmTransaction(
      'createAccount and InitializeAccount',
      this.connection,
      transaction,
      this.payer,
      mintAccount,
    );

    return mintAccount.publicKey;
  }

  /**
   * Create and initializes a new multisig.
   *
   * This account may then be used for multisignature verification
   *
   * @param owner User account that will own the multsig account
   * @return Public key of the new multisig account
   */
  async createMultisig(
    m: number,
    signers: Array<PublicKey>,
  ): Promise<PublicKey> {
    const multisigAccount = new Account();
    let transaction;

    // Allocate memory for the account
    const balanceNeeded = await Token.getMinBalanceRentForExemptMultisig(
      this.connection,
    );
    transaction = SystemProgram.createAccount({
      fromPubkey: this.payer.publicKey,
      newAccountPubkey: multisigAccount.publicKey,
      lamports: balanceNeeded,
      space: MultisigLayout.span,
      programId: this.programId,
    });

    // create the new account
    let keys = [
      {pubkey: multisigAccount.publicKey, isSigner: false, isWritable: true},
    ];
    signers.forEach(signer =>
      keys.push({pubkey: signer, isSigner: false, isWritable: false}),
    );
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      BufferLayout.u8('m'),
    ]);
    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 2, // InitializeM<ultisig instruction
        m,
      },
      data,
    );
    transaction.add({
      keys,
      programId: this.programId,
      data,
    });

    // Send the two instructions
    await sendAndConfirmTransaction(
      'createAccount and InitializeMultisig',
      this.connection,
      transaction,
      this.payer,
      multisigAccount,
    );

    return multisigAccount.publicKey;
  }

  /**
   * Retrieve mint information
   */
  async getMintInfo(): Promise<MintInfo> {
    const info = await this.connection.getAccountInfo(this.publicKey);
    if (info === null) {
      throw new Error('Failed to find mint account');
    }
    if (!info.owner.equals(this.programId)) {
      throw new Error(`Invalid mint owner: ${JSON.stringify(info.owner)}`);
    }
    if (info.data.length != MintLayout.span) {
      throw new Error(`Invalid mint size`);
    }

    const data = Buffer.from(info.data);
    const mintInfo = MintLayout.decode(data);
    if (mintInfo.option === 0) {
      mintInfo.owner = null;
    } else {
      mintInfo.owner = new PublicKey(mintInfo.owner);
    }
    return mintInfo;
  }

  /**
   * Retrieve account information
   *
   * @param account Public key of the account
   */
  async getAccountInfo(account: PublicKey): Promise<AccountInfo> {
    const info = await this.connection.getAccountInfo(account);
    if (info === null) {
      throw new Error('Failed to find account');
    }
    if (!info.owner.equals(this.programId)) {
      throw new Error(`Invalid account owner`);
    }
    if (info.data.length != AccountLayout.span) {
      throw new Error(`Invalid account size`);
    }

    const data = Buffer.from(info.data);
    const accountInfo = AccountLayout.decode(data);
    accountInfo.mint = new PublicKey(accountInfo.mint);
    accountInfo.owner = new PublicKey(accountInfo.owner);
    accountInfo.amount = u64.fromBuffer(accountInfo.amount);
    accountInfo.isInitialized = accountInfo.isInitialized != 0;
    accountInfo.isNative = accountInfo.isNative != 0;
    if (accountInfo.option === 0) {
      accountInfo.delegate = null;
      accountInfo.delegatedAmount = new u64();
    } else {
      accountInfo.delegate = new PublicKey(accountInfo.delegate);
      accountInfo.delegatedAmount = u64.fromBuffer(accountInfo.delegatedAmount);
    }

    if (!accountInfo.mint.equals(this.publicKey)) {
      throw new Error(
        `Invalid account mint: ${JSON.stringify(
          accountInfo.mint,
        )} !== ${JSON.stringify(this.publicKey)}`,
      );
    }
    return accountInfo;
  }

  /**
   * Retrieve Multisig information
   *
   * @param multisig Public key of the account
   */
  async getMultisigInfo(multisig: PublicKey): Promise<MultisigInfo> {
    const info = await this.connection.getAccountInfo(multisig);
    if (info === null) {
      throw new Error('Failed to find multisig');
    }
    if (!info.owner.equals(this.programId)) {
      throw new Error(`Invalid multisig owner`);
    }
    if (info.data.length != MultisigLayout.span) {
      throw new Error(`Invalid multisig size`);
    }

    const data = Buffer.from(info.data);
    const multisigInfo = MultisigLayout.decode(data);
    multisigInfo.signer1 = new PublicKey(multisigInfo.signer1);
    multisigInfo.signer2 = new PublicKey(multisigInfo.signer2);
    multisigInfo.signer3 = new PublicKey(multisigInfo.signer3);
    multisigInfo.signer4 = new PublicKey(multisigInfo.signer4);
    multisigInfo.signer5 = new PublicKey(multisigInfo.signer5);
    multisigInfo.signer6 = new PublicKey(multisigInfo.signer6);
    multisigInfo.signer7 = new PublicKey(multisigInfo.signer7);
    multisigInfo.signer8 = new PublicKey(multisigInfo.signer8);
    multisigInfo.signer9 = new PublicKey(multisigInfo.signer9);
    multisigInfo.signer10 = new PublicKey(multisigInfo.signer10);
    multisigInfo.signer11 = new PublicKey(multisigInfo.signer11);

    return multisigInfo;
  }

  /**
   * Transfer tokens to another account
   *
   * @param source Source account
   * @param destination Destination account
   * @param authority Owner of the source account
   * @param multiSigners Signing accounts if `authority` is a multiSig
   * @param amount Number of tokens to transfer
   */
  async transfer(
    source: PublicKey,
    destination: PublicKey,
    authority: any,
    multiSigners: Array<Account>,
    amount: number | u64,
  ): Promise<TransactionSignature> {
    let ownerPublicKey;
    let signers;
    if (isAccount(authority)) {
      ownerPublicKey = authority.publicKey;
      signers = [authority];
    } else {
      ownerPublicKey = authority;
      signers = multiSigners;
    }
    return await sendAndConfirmTransaction(
      'Transfer',
      this.connection,
      new Transaction().add(
        Token.createTransferInstruction(
          this.programId,
          source,
          destination,
          ownerPublicKey,
          multiSigners,
          amount,
        ),
      ),
      this.payer,
      ...signers,
    );
  }

  /**
   * Grant a third-party permission to transfer up the specified number of tokens from an account
   *
   * @param account Public key of the account
   * @param delegate Account authorized to perform a transfer tokens from the source account
   * @param owner Owner of the source account
   * @param multiSigners Signing accounts if `owner` is a multiSig
   * @param amount Maximum number of tokens the delegate may transfer
   */
  async approve(
    account: PublicKey,
    delegate: PublicKey,
    owner: any,
    multiSigners: Array<Account>,
    amount: number | u64,
  ): Promise<void> {
    let ownerPublicKey;
    let signers;
    if (isAccount(owner)) {
      ownerPublicKey = owner.publicKey;
      signers = [owner];
    } else {
      ownerPublicKey = owner;
      signers = multiSigners;
    }
    await sendAndConfirmTransaction(
      'Approve',
      this.connection,
      new Transaction().add(
        Token.createApproveInstruction(
          this.programId,
          account,
          delegate,
          ownerPublicKey,
          multiSigners,
          amount,
        ),
      ),
      this.payer,
      ...signers,
    );
  }

  /**
   * Remove approval for the transfer of any remaining tokens
   *
   * @param account Public key of the account
   * @param owner Owner of the source account
   * @param multiSigners Signing accounts if `owner` is a multiSig
   */
  async revoke(
    account: PublicKey,
    owner: any,
    multiSigners: Array<Account>,
  ): Promise<void> {
    let ownerPublicKey;
    let signers;
    if (isAccount(owner)) {
      ownerPublicKey = owner.publicKey;
      signers = [owner];
    } else {
      ownerPublicKey = owner;
      signers = multiSigners;
    }
    await sendAndConfirmTransaction(
      'Revoke',
      this.connection,
      new Transaction().add(
        Token.createRevokeInstruction(
          this.programId,
          account,
          ownerPublicKey,
          multiSigners,
        ),
      ),
      this.payer,
      ...signers,
    );
  }

  /**
   * Assign a new owner to the account
   *
   * @param account Public key of the account
   * @param newOwner New owner of the account
   * @param owner Owner of the account
   * @param multiSigners Signing accounts if `owner` is a multiSig
   */
  async setOwner(
    owned: PublicKey,
    newOwner: PublicKey,
    owner: any,
    multiSigners: Array<Account>,
  ): Promise<void> {
    let ownerPublicKey;
    let signers;
    if (isAccount(owner)) {
      ownerPublicKey = owner.publicKey;
      signers = [owner];
    } else {
      ownerPublicKey = owner;
      signers = multiSigners;
    }
    await sendAndConfirmTransaction(
      'SetOwner',
      this.connection,
      new Transaction().add(
        Token.createSetOwnerInstruction(
          this.programId,
          owned,
          newOwner,
          ownerPublicKey,
          multiSigners,
        ),
      ),
      this.payer,
      ...signers,
    );
  }

  /**
   * Mint new tokens
   *
   * @param mint Public key of the mint
   * @param dest Public key of the account to mint to
   * @param authority Owner of the mint
   * @param multiSigners Signing accounts if `authority` is a multiSig
   * @param amount ammount to mint
   */
  async mintTo(
    dest: PublicKey,
    authority: any,
    multiSigners: Array<Account>,
    amount: number,
  ): Promise<void> {
    let ownerPublicKey;
    let signers;
    if (isAccount(authority)) {
      ownerPublicKey = authority.publicKey;
      signers = [authority];
    } else {
      ownerPublicKey = authority;
      signers = multiSigners;
    }
    await sendAndConfirmTransaction(
      'MintTo',
      this.connection,
      new Transaction().add(
        Token.createMintToInstruction(
          this.programId,
          this.publicKey,
          dest,
          ownerPublicKey,
          multiSigners,
          amount,
        ),
      ),
      this.payer,
      ...signers,
    );
  }

  /**
   * Burn tokens
   *
   * @param account Account to burn tokens from
   * @param authority Public key account owner
   * @param multiSigners Signing accounts if `authority` is a multiSig
   * @param amount ammount to burn
   */
  async burn(
    account: PublicKey,
    authority: any,
    multiSigners: Array<Account>,
    amount: number,
  ): Promise<void> {
    let ownerPublicKey;
    let signers;
    if (isAccount(authority)) {
      ownerPublicKey = authority.publicKey;
      signers = [authority];
    } else {
      ownerPublicKey = authority;
      signers = multiSigners;
    }
    await sendAndConfirmTransaction(
      'Burn',
      this.connection,
      new Transaction().add(
        Token.createBurnInstruction(
          this.programId,
          account,
          ownerPublicKey,
          multiSigners,
          amount,
        ),
      ),
      this.payer,
      ...signers,
    );
  }

  /**
   * Burn account
   *
   * @param account Account to burn
   * @param authority account owner
   * @param multiSigners Signing accounts if `owner` is a multiSig
   */
  async closeAccount(
    account: PublicKey,
    dest: PublicKey,
    owner: any,
    multiSigners: Array<Account>,
  ): Promise<void> {
    let ownerPublicKey;
    let signers;
    if (isAccount(owner)) {
      ownerPublicKey = owner.publicKey;
      signers = [owner];
    } else {
      ownerPublicKey = owner;
      signers = multiSigners;
    }
    await sendAndConfirmTransaction(
      'CloseAccount',
      this.connection,
      new Transaction().add(
        Token.createCloseAccountInstruction(
          this.programId,
          account,
          dest,
          ownerPublicKey,
          multiSigners,
        ),
      ),
      this.payer,
      ...signers,
    );
  }

  /**
   * Construct a Transfer instruction
   *
   * @param source Source account
   * @param destination Destination account
   * @param authority Owner of the source account
   * @param multiSigners Signing accounts if `authority` is a multiSig
   * @param amount Number of tokens to transfer
   */
  static createTransferInstruction(
    programId: PublicKey,
    source: PublicKey,
    destination: PublicKey,
    authority: any,
    multiSigners: Array<Account>,
    amount: number | u64,
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('amount'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 3, // Transfer instruction
        amount: new u64(amount).toBuffer(),
      },
      data,
    );

    let keys = [
      {pubkey: source, isSigner: false, isWritable: true},
      {pubkey: destination, isSigner: false, isWritable: true},
    ];
    if (isAccount(authority)) {
      keys.push({
        pubkey: authority.publicKey,
        isSigner: true,
        isWritable: false,
      });
    } else {
      keys.push({pubkey: authority, isSigner: false, isWritable: false});
      multiSigners.forEach(signer =>
        keys.push({
          pubkey: signer.publicKey,
          isSigner: true,
          isWritable: false,
        }),
      );
    }
    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }

  /**
   * Construct an Approve instruction
   *
   * @param account Public key of the account
   * @param delegate Account authorized to perform a transfer of tokens from the source account
   * @param owner Owner of the source account
   * @param multiSigners Signing accounts if `owner` is a multiSig
   * @param amount Maximum number of tokens the delegate may transfer
   */
  static createApproveInstruction(
    programId: PublicKey,
    account: PublicKey,
    delegate: PublicKey,
    owner: any,
    multiSigners: Array<Account>,
    amount: number | u64,
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('amount'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 4, // Approve instruction
        amount: new u64(amount).toBuffer(),
      },
      data,
    );

    let keys = [
      {pubkey: account, isSigner: false, isWritable: true},
      {pubkey: delegate, isSigner: false, isWritable: false},
    ];
    if (isAccount(owner)) {
      keys.push({pubkey: owner.publicKey, isSigner: true, isWritable: false});
    } else {
      keys.push({pubkey: owner, isSigner: false, isWritable: false});
      multiSigners.forEach(signer =>
        keys.push({
          pubkey: signer.publicKey,
          isSigner: true,
          isWritable: false,
        }),
      );
    }

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }

  /**
   * Construct an Approve instruction
   *
   * @param account Public key of the account
   * @param delegate Account authorized to perform a transfer of tokens from the source account
   * @param owner Owner of the source account
   * @param multiSigners Signing accounts if `owner` is a multiSig
   * @param amount Maximum number of tokens the delegate may transfer
   */
  static createRevokeInstruction(
    programId: PublicKey,
    account: PublicKey,
    owner: any,
    multiSigners: Array<Account>,
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([BufferLayout.u8('instruction')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 5, // Approve instruction
      },
      data,
    );

    let keys = [{pubkey: account, isSigner: false, isWritable: true}];
    if (isAccount(owner)) {
      keys.push({pubkey: owner.publicKey, isSigner: true, isWritable: false});
    } else {
      keys.push({pubkey: owner, isSigner: false, isWritable: false});
      multiSigners.forEach(signer =>
        keys.push({
          pubkey: signer.publicKey,
          isSigner: true,
          isWritable: false,
        }),
      );
    }

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }

  /**
   * Construct a SetOwner instruction
   *
   * @param account Public key of the account
   * @param newOwner New owner of the account
   * @param owner Owner of the account
   * @param multiSigners Signing accounts if `owner` is a multiSig
   */
  static createSetOwnerInstruction(
    programId: PublicKey,
    owned: PublicKey,
    newOwner: PublicKey,
    owner: any,
    multiSigners: Array<Account>,
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([BufferLayout.u8('instruction')]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 6, // SetOwner instruction
      },
      data,
    );

    let keys = [
      {pubkey: owned, isSigner: false, isWritable: true},
      {pubkey: newOwner, isSigner: false, isWritable: false},
    ];
    if (isAccount(owner)) {
      keys.push({pubkey: owner.publicKey, isSigner: true, isWritable: false});
    } else {
      keys.push({pubkey: owner, isSigner: false, isWritable: false});
      multiSigners.forEach(signer =>
        keys.push({
          pubkey: signer.publicKey,
          isSigner: true,
          isWritable: false,
        }),
      );
    }

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }

  /**
   * Construct a MintTo instruction
   *
   * @param dest Public key of the account to mint to
   * @param authority Owner of the mint
   * @param multiSigners Signing accounts if `authority` is a multiSig

   * @param amount amount to mint
   */
  static createMintToInstruction(
    programId: PublicKey,
    mint: PublicKey,
    dest: PublicKey,
    authority: any,
    multiSigners: Array<Account>,
    amount: number,
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('amount'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 7, // MintTo instruction
        amount: new u64(amount).toBuffer(),
      },
      data,
    );

    let keys = [
      {pubkey: mint, isSigner: false, isWritable: true},
      {pubkey: dest, isSigner: false, isWritable: true},
    ];
    if (isAccount(authority)) {
      keys.push({
        pubkey: authority.publicKey,
        isSigner: true,
        isWritable: false,
      });
    } else {
      keys.push({pubkey: authority, isSigner: false, isWritable: false});
      multiSigners.forEach(signer =>
        keys.push({
          pubkey: signer.publicKey,
          isSigner: true,
          isWritable: false,
        }),
      );
    }

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }

  /**
   * Construct a Burn instruction
   *
   * @param account Account to burn tokens from
   * @param authority Public key account owner
   * @param multiSigners Signing accounts if `authority` is a multiSig
   * @param amount ammount to burn
   */
  static createBurnInstruction(
    programId: PublicKey,
    account: PublicKey,
    authority: any,
    multiSigners: Array<Account>,
    amount: number,
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      Layout.uint64('amount'),
    ]);

    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 8, // Burn instruction
        amount: new u64(amount).toBuffer(),
      },
      data,
    );

    let keys = [{pubkey: account, isSigner: false, isWritable: true}];
    if (isAccount(authority)) {
      keys.push({
        pubkey: authority.publicKey,
        isSigner: true,
        isWritable: false,
      });
    } else {
      keys.push({pubkey: authority, isSigner: false, isWritable: false});
      multiSigners.forEach(signer =>
        keys.push({
          pubkey: signer.publicKey,
          isSigner: true,
          isWritable: false,
        }),
      );
    }

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }

  /**
   * Construct a Burn instruction
   *
   * @param account Account to burn tokens from
   * @param owner account owner
   * @param multiSigners Signing accounts if `owner` is a multiSig
   */
  static createCloseAccountInstruction(
    programId: PublicKey,
    account: PublicKey,
    dest: PublicKey,
    owner: any,
    multiSigners: Array<Account>,
  ): TransactionInstruction {
    const dataLayout = BufferLayout.struct([BufferLayout.u8('instruction')]);
    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: 9, // CloseAccount instruction
      },
      data,
    );

    let keys = [
      {pubkey: account, isSigner: false, isWritable: true},
      {pubkey: dest, isSigner: false, isWritable: true},
    ];
    if (isAccount(owner)) {
      keys.push({pubkey: owner.publicKey, isSigner: true, isWritable: false});
    } else {
      keys.push({pubkey: owner, isSigner: false, isWritable: false});
      multiSigners.forEach(signer =>
        keys.push({
          pubkey: signer.publicKey,
          isSigner: true,
          isWritable: false,
        }),
      );
    }

    return new TransactionInstruction({
      keys,
      programId: programId,
      data,
    });
  }
}
