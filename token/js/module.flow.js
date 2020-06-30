/**
 * Flow Library definition for token
 *
 * This file is manually maintained
 *
 * Usage: add the following line under the [libs] section of your project's
 * .flowconfig:
 * [libs]
 * token/module.flow.js
 *
 */

declare module 'spl-token' {
  // === client/token.js ===
  declare export class TokenAmount extends BN {
    /**
     * Convert to Buffer representation
     */
    toBuffer(): Buffer;
    static fromBuffer(buffer: Buffer): TokenAmount;
  }
  declare export class Token {
    constructor(
      connection: Connection,
      publicKey: PublicKey,
      programId: PublicKey,
      payer: Account,
    ): Token;
    static createNewToken(
      connection: Connection,
      payer: Account,
      owner: Account,
      supply: TokenAmount,
      decimals: number,
      programId: PublicKey,
      is_owned: boolean,
    ): Promise<TokenAndPublicKey>;
    static getAccount(connection: Connection): Promise<Account>;
    newAccount(owner: Account, source: null | PublicKey): Promise<PublicKey>;
    getTokenInfo(): Promise<TokenInfo>;
    getTokenAccountInfo(account: PublicKey): Promise<TokenAccountInfo>;
  }
}
