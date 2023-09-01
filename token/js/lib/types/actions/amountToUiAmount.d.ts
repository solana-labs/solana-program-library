import type { Connection, PublicKey, Signer, TransactionError } from '@solana/web3.js';
/**
 * Amount as a string using mint-prescribed decimals
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction fees
 * @param mint           Mint for the account
 * @param amount         Amount of tokens to be converted to Ui Amount
 * @param programId      SPL Token program account
 *
 * @return Ui Amount generated
 */
export declare function amountToUiAmount(connection: Connection, payer: Signer, mint: PublicKey, amount: number | bigint, programId?: PublicKey): Promise<string | TransactionError | null>;
//# sourceMappingURL=amountToUiAmount.d.ts.map