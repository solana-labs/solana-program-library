import { Connection, PublicKey, sendAndConfirmTransaction, Signer, Transaction } from '@solana/web3.js';
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from '../constants';
import { TokenError } from '../errors';
import { createAssociatedTokenAccountInstruction } from '../instructions';
import { Account, getAccountInfo, getAssociatedTokenAddress } from '../state';

/**
 * Retrieve the associated account, or create one if not found
 * This account may then be used as a `transfer()` or `approve()` destination
 *
 * @param owner User account that will own the new account
 * @param @TODO: docs
 *
 * @return The new associated account
 */
export async function getOrCreateAssociatedTokenAccount(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    owner: PublicKey,
    programId = ASSOCIATED_TOKEN_PROGRAM_ID,
    tokenProgramId = TOKEN_PROGRAM_ID
): Promise<Account> {
    const associatedAddress = await getAssociatedTokenAddress(mint, owner, false, programId, tokenProgramId);

    // This is the optimal logic, considering TX fee, client-side computation, RPC roundtrips and guaranteed idempotent.
    // Sadly we can't do this atomically.
    let account: Account;
    try {
        account = await getAccountInfo(connection, associatedAddress);
    } catch (err: any) {
        // INVALID_ACCOUNT_OWNER can be possible if the associated address has already been received some lamports,
        // becoming system accounts. Assuming program derived addressing is safe, this is the only case for the
        // INVALID_ACCOUNT_OWNER in this code-path.
        if (err?.message === TokenError.ACCOUNT_NOT_FOUND || err?.message === TokenError.INVALID_ACCOUNT_OWNER) {
            // As this isn't atomic, it's possible others can create associated accounts meanwhile.
            try {
                const transaction = new Transaction().add(
                    createAssociatedTokenAccountInstruction(
                        programId,
                        tokenProgramId,
                        mint,
                        associatedAddress,
                        owner,
                        payer.publicKey
                    )
                );

                await sendAndConfirmTransaction(connection, transaction, [payer]);
            } catch (err: any) {
                // Ignore all errors; for now there is no API compatible way to selectively ignore the expected
                // instruction error if the associated account is existing already.
            }

            // Now this should always succeed
            account = await getAccountInfo(connection, associatedAddress);
        } else {
            throw err;
        }
    }

    if (!account.mint.equals(mint)) throw new Error(TokenError.INVALID_MINT);
    return account;
}
