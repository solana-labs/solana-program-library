/**
 * Flow Library definition for spl-token
 *
 * This file is manually maintained
 *
 */

declare module '@solana/spl-token' {
  // === client/token.js ===
  declare export class u64 extends BN {
    toBuffer(): Buffer;
    static fromBuffer(buffer: Buffer): u64;
  }
  declare export type MintInfo = {|
    owner: null | PublicKey,
    decimals: number,
    initialized: boolean,
  |};
  declare export type AccountInfo = {|
    mint: PublicKey,
    owner: PublicKey,
    amount: u64,
    delegate: null | PublicKey,
    delegatedAmount: u64,
    isInitialized: boolean,
    isNative: boolean,
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
  declare export type TokenAndPublicKey = [Token, PublicKey];
  declare export class Token {
    constructor(
      connection: Connection,
      publicKey: PublicKey,
      programId: PublicKey,
      payer: Account,
    ): Token;
    static createMint(
      connection: Connection,
      payer: Account,
      mintOwner: PublicKey,
      accountOwner: PublicKey,
      supply: u64,
      decimals: number,
      programId: PublicKey,
      is_owned: boolean,
    ): Promise<TokenAndPublicKey>;
    static getAccount(connection: Connection): Promise<Account>;
    createAccount(owner: PublicKey): Promise<PublicKey>;
    createMultisig(m: number, signers: Array<PublicKey>): Promise<PublicKey>;
    getMintInfo(): Promise<MintInfo>;
    getAccountInfo(account: PublicKey): Promise<AccountInfo>;
    getMultisigInfo(multisig: PublicKey): Promise<MultisigInfo>;
    transfer(
      source: PublicKey,
      destination: PublicKey,
      authority: Account | PublicKey,
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
    setOwner(
      owned: PublicKey,
      newOwner: PublicKey,
      owner: Account | PublicKey,
      multiSigners: Array<Account>,
    ): Promise<void>;
    mintTo(
      dest: PublicKey,
      authority: Account | PublicKey,
      multiSigners: Array<Account>,
      amount: number,
    ): Promise<void>;
    burn(
      account: PublicKey,
      authority: Account | PublicKey,
      multiSigners: Array<Account>,
      amount: number,
    ): Promise<void>;
    closeAccount(
      account: PublicKey,
      dest: PublicKey,
      owner: Account | PublicKey,
      multiSigners: Array<Account>,
    ): Promise<void>;
    static createTransferInstruction(
      programId: PublicKey,
      source: PublicKey,
      destination: PublicKey,
      authority: Account | PublicKey,
      multiSigners: Array<Account>,
      amount: number | u64,
    ): TransactionInstruction;
    static createApproveInstruction(
      programId: PublicKey,
      account: PublicKey,
      delegate: PublicKey,
      owner: Account | PublicKey,
      multiSigners: Array<Account>,
      amount: number | u64,
    ): TransactionInstruction;
    static createRevokeInstruction(
      programId: PublicKey,
      account: PublicKey,
      owner: Account | PublicKey,
      multiSigners: Array<Account>,
    ): TransactionInstruction;
    static createSetOwnerInstruction(
      programId: PublicKey,
      owned: PublicKey,
      newOwner: PublicKey,
      owner: Account | PublicKey,
      multiSigners: Array<Account>,
    ): TransactionInstruction;
    static createMintToInstruction(
      programId: PublicKey,
      mint: PublicKey,
      dest: PublicKey,
      authority: Account | PublicKey,
      multiSigners: Array<Account>,
      amount: number,
    ): TransactionInstruction;
    static createBurnInstruction(
      programId: PublicKey,
      account: PublicKey,
      authority: Account | PublicKey,
      multiSigners: Array<Account>,
      amount: number,
    ): TransactionInstruction;
    static createCloseAccountInstruction(
      programId: PublicKey,
      account: PublicKey,
      dest: PublicKey,
      owner: Account | PublicKey,
      multiSigners: Array<Account>,
    ): TransactionInstructio;
  }
}
