/**
 * Flow Library definition for spl-token
 *
 * This file is manually maintained
 */

import BN from 'bn.js'; // eslint-disable-line
import {Buffer} from 'buffer';
import {Layout} from 'buffer-layout';
import {
  Account,
  Connection,
  PublicKey,
  TransactionInstruction,
} from '@solana/web3.js';
import type {TransactionSignature} from '@solana/web3.js';

declare module '@solana/spl-token' {
  declare export var TOKEN_PROGRAM_ID;
  declare export var ASSOCIATED_TOKEN_PROGRAM_ID;
  declare export class u64 extends BN {
    toBuffer(): typeof Buffer;
    static fromBuffer(buffer: typeof Buffer): u64;
  }
  declare export type AuthorityType =
    | 'MintTokens'
    | 'FreezeAccount'
    | 'AccountOwner'
    | 'CloseAccount';
  declare export var NATIVE_MINT: PublicKey;
  declare export var MintLayout: typeof Layout;
  declare export type MintInfo = {|
    mintAuthority: null | PublicKey,
    supply: u64,
    decimals: number,
    isInitialized: boolean,
    freezeAuthority: null | PublicKey,
  |};
  declare export var AccountLayout: typeof Layout;
  declare export type AccountInfo = {|
    address: PublicKey,
    mint: PublicKey,
    owner: PublicKey,
    amount: u64,
    delegate: null | PublicKey,
    delegatedAmount: u64,
    isInitialized: boolean,
    isFrozen: boolean,
    isNative: boolean,
    rentExemptReserve: null | u64,
    closeAuthority: null | PublicKey,
  |};
  declare export type MultisigInfo = {|
    m: number,
    n: number,
    initialized: boolean,
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
  declare export class Token {
    publicKey: PublicKey;
    programId: PublicKey;
    associatedProgramId: PublicKey;
    payer: Account;
    constructor(
      connection: Connection,
      publicKey: PublicKey,
      programId: PublicKey,
      payer: Account,
    ): Token;
    static getMinBalanceRentForExemptMint(
      connection: Connection,
    ): Promise<number>;
    static getMinBalanceRentForExemptAccount(
      connection: Connection,
    ): Promise<number>;
    static getMinBalanceRentForExemptMultisig(
      connection: Connection,
    ): Promise<number>;
    static getAssociatedTokenAddress(
      associatedProgramId: PublicKey,
      programId: PublicKey,
      mint: PublicKey,
      owner: PublicKey,
    ): Promise<PublicKey>;
    static createMint(
      connection: Connection,
      payer: Account,
      mintAuthority: PublicKey,
      freezeAuthority: PublicKey | null,
      decimals: number,
      programId: PublicKey,
    ): Promise<Token>;
    createAccount(owner: PublicKey): Promise<PublicKey>;
    createAssociatedTokenAccount(owner: PublicKey): Promise<PublicKey>;
    static createWrappedNativeAccount(
      connection: Connection,
      programId: PublicKey,
      owner: PublicKey,
      payer: Account,
      amount: number,
    ): Promise<PublicKey>;
    createMultisig(m: number, signers: Array<PublicKey>): Promise<PublicKey>;
    getMintInfo(): Promise<MintInfo>;
    getAccountInfo(account: PublicKey): Promise<AccountInfo>;
    getOrCreateAssociatedAccountInfo(owner: PublicKey): Promise<AccountInfo>;
    getMultisigInfo(multisig: PublicKey): Promise<MultisigInfo>;
    transfer(
      source: PublicKey,
      destination: PublicKey,
      owner: Account | PublicKey,
      multiSigners: Array<Account>,
      amount: number | u64,
    ): Promise<TransactionSignature>;
    approve(
      account: PublicKey,
      delegate: PublicKey,
      owner: Account | PublicKey,
      multiSigners: Array<Account>,
      amount: number | u64,
    ): Promise<void>;
    revoke(
      account: PublicKey,
      owner: Account | PublicKey,
      multiSigners: Array<Account>,
    ): Promise<void>;
    setAuthority(
      account: PublicKey,
      newAuthority: PublicKey | null,
      authorityType: AuthorityType,
      currentAuthority: Account | PublicKey,
      multiSigners: Array<Account>,
    ): Promise<void>;
    mintTo(
      dest: PublicKey,
      authority: Account | PublicKey,
      multiSigners: Array<Account>,
      amount: number | u64,
    ): Promise<void>;
    burn(
      account: PublicKey,
      owner: Account | PublicKey,
      multiSigners: Array<Account>,
      amount: number | u64,
    ): Promise<void>;
    freezeAccount(
      account: PublicKey,
      authority: any,
      multiSigners: Array<Account>,
    ): Promise<void>;
    thawAccount(
      account: PublicKey,
      authority: any,
      multiSigners: Array<Account>,
    ): Promise<void>;
    closeAccount(
      account: PublicKey,
      dest: PublicKey,
      authority: Account | PublicKey,
      multiSigners: Array<Account>,
    ): Promise<void>;
    static createInitMintInstruction(
      programId: PublicKey,
      mint: PublicKey,
      decimals: number,
      mintAuthority: PublicKey,
      freezeAuthority: PublicKey | null,
    ): TransactionInstruction;
    static createInitAccountInstruction(
      programId: PublicKey,
      mint: PublicKey,
      account: PublicKey,
      owner: PublicKey,
    ): TransactionInstruction;
    static createTransferInstruction(
      programId: PublicKey,
      source: PublicKey,
      destination: PublicKey,
      owner: PublicKey,
      multiSigners: Array<Account>,
      amount: number | u64,
    ): TransactionInstruction;
    static createApproveInstruction(
      programId: PublicKey,
      account: PublicKey,
      delegate: PublicKey,
      owner: PublicKey,
      multiSigners: Array<Account>,
      amount: number | u64,
    ): TransactionInstruction;
    static createRevokeInstruction(
      programId: PublicKey,
      account: PublicKey,
      owner: PublicKey,
      multiSigners: Array<Account>,
    ): TransactionInstruction;
    static createSetAuthorityInstruction(
      programId: PublicKey,
      account: PublicKey,
      newAuthority: PublicKey | null,
      authorityType: AuthorityType,
      authority: PublicKey,
      multiSigners: Array<Account>,
    ): TransactionInstruction;
    static createMintToInstruction(
      programId: PublicKey,
      mint: PublicKey,
      dest: PublicKey,
      authority: PublicKey,
      multiSigners: Array<Account>,
      amount: number | u64,
    ): TransactionInstruction;
    static createBurnInstruction(
      programId: PublicKey,
      mint: PublicKey,
      account: PublicKey,
      owner: PublicKey,
      multiSigners: Array<Account>,
      amount: number | u64,
    ): TransactionInstruction;
    static createCloseAccountInstruction(
      programId: PublicKey,
      account: PublicKey,
      dest: PublicKey,
      authority: PublicKey,
      multiSigners: Array<Account>,
    ): TransactionInstruction;
    static createFreezeAccountInstruction(
      programId: PublicKey,
      account: PublicKey,
      mint: PublicKey,
      authority: PublicKey,
      multiSigners: Array<Account>,
    ): TransactionInstruction;
    static createThawAccountInstruction(
      programId: PublicKey,
      account: PublicKey,
      mint: PublicKey,
      authority: PublicKey,
      multiSigners: Array<Account>,
    ): TransactionInstruction;
    static createAssociatedTokenAccountInstruction(
      associatedProgramId: PublicKey,
      programId: PublicKey,
      mint: PublicKey,
      associatedAccount: PublicKey,
      owner: PublicKey,
      payer: PublicKey,
    ): TransactionInstruction;
  }
}
