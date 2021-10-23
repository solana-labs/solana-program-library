import {
    Connection,
    Keypair,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    SystemProgram,
    Transaction,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createInitializeAccountInstruction } from '../instructions';
import { ACCOUNT_LEN, getMinimumBalanceForRentExemptAccount } from '../state';

/**
 * Create and initialize a new token account
 *
 * This account may then be used as a `transfer()` or `approve()` destination
 *
 * @param owner User account that will own the new account
 *
 * @return Address the new token account
 */
export async function createAccount(
    connection: Connection,
    mint: PublicKey,
    owner: PublicKey,
    payer: Signer,
    programId = TOKEN_PROGRAM_ID
): Promise<PublicKey> {
    const lamports = await getMinimumBalanceForRentExemptAccount(connection);

    const newAccount = Keypair.generate();

    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: newAccount.publicKey,
            space: ACCOUNT_LEN,
            lamports,
            programId,
        }),
        createInitializeAccountInstruction(mint, newAccount.publicKey, owner, programId)
    );

    await sendAndConfirmTransaction(connection, transaction, [payer, newAccount]);

    return newAccount.publicKey;
}
