import {
    ConfirmOptions,
    Connection,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    Transaction,
    TransactionSignature,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createBurnCheckedInstruction } from '../instructions';
import { getSigners } from './internal';

/**
 * Burn tokens from an account, asserting the token mint and decimals
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction fees
 * @param account        Account to burn tokens from
 * @param mint           Mint for the account
 * @param owner          Account owner
 * @param multiSigners   Signing accounts if `owner` is a multisig
 * @param amount         Amount to burn
 * @param decimals       Number of decimals in amount to burn
 * @param confirmOptions Options for confirming the transaction
 * @param programId      SPL Token program account
 *
 * @return Signature of the confirmed transaction
 */
export async function burnChecked(
    connection: Connection,
    payer: Signer,
    account: PublicKey,
    mint: PublicKey,
    owner: Signer | PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    decimals: number,
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    const [ownerPublicKey, signers] = getSigners(owner, multiSigners);

    const transaction = new Transaction().add(
        createBurnCheckedInstruction(account, mint, ownerPublicKey, multiSigners, amount, decimals, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers], confirmOptions);
}
