import assert from 'assert';
import BN from 'bn.js';
import BufferLayout from 'buffer-layout';
import {
  Account,
  PublicKey,
  SystemProgram,
  Transaction,
  TransactionInstruction,
  SYSVAR_RENT_PUBKEY,
} from '@solana/web3.js';
import { Connection, Commitment } from '@solana/web3.js';

import * as Layout from './layout';
import { sendAndConfirmTransaction } from './util/send-and-confirm-transaction';

enum Instruction {
  INITIALIZE_IDENTITY,
}

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
      Array.from(buffer)
        .reverse()
        .map(i => `00${i.toString(16)}`.slice(-2))
        .join(''),
      16
    );
  }
}

// The address of the special mint for wrapped native identity.
export const NATIVE_MINT: PublicKey = new PublicKey(
  'So11111111111111111111111111111111111111112'
);

/**
 * Information about an account
 */
type AccountInfo = {
  /**
   * Owner of this account
   */
  owner: PublicKey;
};

/**
 * @private
 */
export const IdentityAccountLayout: BufferLayout.Layout = BufferLayout.struct([
  Layout.publicKey('owner'),
  BufferLayout.u8('state'),
]);

/**
 * An Identity Account
 */
export class Identity {
  /**
   * @private
   */
  connection: Connection;

  /**
   * Program Identifier for the Identity program
   */
  programId: PublicKey;

  /**
   * Fee payer
   */
  payer: Account;

  /**
   * Create an identity object
   *
   * @param connection The connection to use
   * @param programId identity programId
   * @param payer Payer of fees
   */
  constructor(connection: Connection, programId: PublicKey, payer: Account) {
    this.connection = connection;
    this.programId = programId;
    this.payer = payer;
  }

  /**
   * Get the minimum balance for the account to be rent exempt
   *
   * @return Number of lamports required
   */
  static async getMinBalanceRentForExemptAccount(
    connection: Connection
  ): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(
      IdentityAccountLayout.span
    );
  }

  /**
   * Create and initialize a new account.
   *
   * @param owner User account that will own the new account
   * @return Public key of the new empty account
   */
  async createAccount(owner: PublicKey): Promise<PublicKey> {
    // Allocate memory for the account
    const balanceNeeded = await Identity.getMinBalanceRentForExemptAccount(
      this.connection
    );

    const newAccount = new Account();
    const transaction = new Transaction();
    transaction.add(
      SystemProgram.createAccount({
        fromPubkey: this.payer.publicKey,
        newAccountPubkey: newAccount.publicKey,
        lamports: balanceNeeded,
        space: IdentityAccountLayout.span,
        programId: this.programId,
      })
    );

    transaction.add(
      Identity.createInitAccountInstruction(
        this.programId,
        newAccount.publicKey,
        owner
      )
    );

    // Send the two instructions
    await sendAndConfirmTransaction(
      'createAccount and InitializeAccount',
      this.connection,
      transaction,
      this.payer,
      newAccount
    );

    return newAccount.publicKey;
  }

  /**
   * Retrieve account information
   *
   * @param account Public key of the account
   * @param commitment
   */
  async getAccountInfo(
    account: PublicKey,
    commitment?: Commitment
  ): Promise<AccountInfo> {
    const info = await this.connection.getAccountInfo(account, commitment);
    if (info === null) {
      throw new Error('Failed to find account');
    }
    if (!info.owner.equals(this.programId)) {
      throw new Error(`Invalid account owner`);
    }
    if (info.data.length !== IdentityAccountLayout.span) {
      throw new Error(`Invalid account size`);
    }

    const data = Buffer.from(info.data);
    const accountInfo = IdentityAccountLayout.decode(data);
    accountInfo.owner = new PublicKey(accountInfo.owner);
    accountInfo.amount = u64.fromBuffer(accountInfo.amount);

    accountInfo.isInitialized = accountInfo.state !== 0;
    accountInfo.isFrozen = accountInfo.state === 2;

    if (accountInfo.isNativeOption === 1) {
      accountInfo.rentExemptReserve = u64.fromBuffer(accountInfo.isNative);
      accountInfo.isNative = true;
    } else {
      accountInfo.rentExemptReserve = null;
      accountInfo.isNative = false;
    }

    if (accountInfo.closeAuthorityOption === 0) {
      accountInfo.closeAuthority = null;
    } else {
      accountInfo.closeAuthority = new PublicKey(accountInfo.closeAuthority);
    }

    return accountInfo;
  }

  /**
   * Construct an InitializeAccount instruction
   *
   * @param programId SPL Identity program account
   * @param account New account
   * @param owner Owner of the new account
   */
  static createInitAccountInstruction(
    programId: PublicKey,
    account: PublicKey,
    owner: PublicKey
  ): TransactionInstruction {
    const keys = [
      { pubkey: account, isSigner: false, isWritable: true },
      { pubkey: owner, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
    ];
    const dataLayout = BufferLayout.struct([BufferLayout.u8('instruction')]);
    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: Instruction.INITIALIZE_IDENTITY,
      },
      data
    );

    return new TransactionInstruction({
      keys,
      programId,
      data,
    });
  }
}
