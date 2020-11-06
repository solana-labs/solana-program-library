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
  ATTEST,
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

enum AccountState {
  Uninitialized,
  Initialized,
}

// the max size of an attestation hash in bytes
export const ATTESTATION_SIZE = 32;
type Attestation = {
  idv: PublicKey;
  attestationData: string;
};

/**
 * Information about an account
 */
export type IdentityAccountInfo = {
  owner: PublicKey;
  state: AccountState;
  attestation?: Attestation;
};

/**
 * @private
 */
export const IdentityAccountLayout: BufferLayout.Layout = BufferLayout.struct([
  Layout.publicKey('owner'), // 32 bytes pubkey
  BufferLayout.u8('state'), // 1 byte enum
  BufferLayout.u8('numAttestations'), // 1 byte unsigned int
  BufferLayout.struct(
    [
      Layout.publicKey('idv'), //  32 bytes idv pubkey
      BufferLayout.blob(ATTESTATION_SIZE, 'attestationData'), // 32 bytes attestation hash
    ],
    'attestation'
  ),
]);

/**
 * An Identity Client
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
      'createAccount and InitializeIdentity',
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
  ): Promise<IdentityAccountInfo> {
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

    return this.accountInfoDataToIdentity(data);
  }

  accountInfoDataToIdentity(data: Buffer) {
    const decodedAccountInfo = IdentityAccountLayout.decode(data);

    const attestation = decodedAccountInfo.numAttestations
      ? {
          idv: new PublicKey(decodedAccountInfo.attestation.idv),
          attestationData: decodedAccountInfo.attestation.attestationData.toString(),
        }
      : undefined;

    return {
      owner: new PublicKey(decodedAccountInfo.owner),
      state: decodedAccountInfo.state,
      attestation,
    };
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

  /**
   * Create and initialize a new account.
   *
   * @param identityAccount The subject the attestation applies to
   * @param idv The IDV making the attestation
   * @param attestation The attestation data
   */
  async attest(
    identityAccount: PublicKey,
    idv: Account,
    attestation: string
  ): Promise<void> {
    const transaction = new Transaction();
    transaction.add(
      Identity.createAttestInstruction(
        this.programId,
        identityAccount,
        idv.publicKey,
        attestation
      )
    );

    // Send the transaction
    await sendAndConfirmTransaction(
      'attest',
      this.connection,
      transaction,
      idv
    );
  }

  /**
   * Construct an Attest instruction
   *
   * @param programId SPL Identity program account
   * @param identityAccount The identity that the attestation belongs to
   * @param idv The IDV making the attestation
   * @param attestation The attestation data
   */
  static createAttestInstruction(
    programId: PublicKey,
    identityAccount: PublicKey,
    idv: PublicKey,
    attestation: string
  ): TransactionInstruction {
    const keys = [
      { pubkey: identityAccount, isSigner: false, isWritable: true },
      { pubkey: idv, isSigner: true, isWritable: false },
    ];
    const dataLayout = BufferLayout.struct([
      BufferLayout.u8('instruction'),
      BufferLayout.blob(ATTESTATION_SIZE, 'attestationData'),
    ]);
    const data = Buffer.alloc(dataLayout.span);
    dataLayout.encode(
      {
        instruction: Instruction.ATTEST,
        attestationData: Buffer.from(attestation, 'utf8'),
      },
      data
    );

    return new TransactionInstruction({
      keys,
      programId,
      data,
    });
  }

  async hasAttestation(
    identityAccount: PublicKey,
    idv: PublicKey,
    attestationData: string
  ): Promise<boolean> {
    const accountInfo = await this.getAccountInfo(identityAccount);

    if (!accountInfo.attestation) return false;
    if (accountInfo.attestation.idv.toBase58() !== idv.toBase58()) return false;

    return accountInfo.attestation.attestationData === attestationData;
  }
}
