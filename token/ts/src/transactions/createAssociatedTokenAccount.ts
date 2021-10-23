import { Connection, PublicKey, sendAndConfirmTransaction, Signer, Transaction } from '@solana/web3.js';
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID } from '../constants';
import { createAssociatedTokenAccountInstruction } from '../instructions';
import { getAssociatedTokenAddress } from '../state';

/**
 * Create and initialize the associated account.
 * This account may then be used as a `transfer()` or `approve()` destination
 *
 * @param owner User account that will own the new account
 * @param @TODO: docs
 *
 * @return Public key of the new associated account
 */
export async function createAssociatedTokenAccount(
    connection: Connection,
    mint: PublicKey,
    owner: PublicKey,
    payer: Signer,
    programId = TOKEN_PROGRAM_ID,
    associatedTokenProgramId = ASSOCIATED_TOKEN_PROGRAM_ID
): Promise<PublicKey> {
    const associatedAddress = await getAssociatedTokenAddress(mint, owner, false, programId, associatedTokenProgramId);

    const transaction = new Transaction().add(
        createAssociatedTokenAccountInstruction(
            associatedTokenProgramId,
            programId,
            mint,
            associatedAddress,
            owner,
            payer.publicKey
        )
    );

    await sendAndConfirmTransaction(connection, transaction, [payer]);

    return associatedAddress;
}
