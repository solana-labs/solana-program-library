import {
    ConfirmOptions,
    Connection,
    Keypair,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    SystemProgram,
    Transaction,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createInitializeAccountInstruction } from '../instructions/index';
import { ACCOUNT_SIZE, getMinimumBalanceForRentExemptAccount } from '../state/index';
import { createAssociatedTokenAccount } from './createAssociatedTokenAccount';

/**
 * Create and initialize a new token account
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction and initialization fees
 * @param mint           Mint for the account
 * @param owner          Owner of the new account
 * @param keypair        Optional keypair, defaulting to the associated token account for the `mint` and `owner`
 * @param confirmOptions Options for confirming the transaction
 * @param programId      SPL Token program account
 *
 * @return Address of the new token account
 */
export async function createAccount(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    owner: PublicKey,
    keypair?: Keypair,
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_PROGRAM_ID
): Promise<PublicKey> {
    // If a keypair isn't provided, create the associated token account and return its address
    if (!keypair) return await createAssociatedTokenAccount(connection, payer, mint, owner, confirmOptions, programId);

    // Otherwise, create the account with the provided keypair and return its public key
    const lamports = await getMinimumBalanceForRentExemptAccount(connection);

    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: keypair.publicKey,
            space: ACCOUNT_SIZE,
            lamports,
            programId,
        }),
        createInitializeAccountInstruction(keypair.publicKey, mint, owner, programId)
    );

    await sendAndConfirmTransaction(connection, transaction, [payer, keypair], confirmOptions);

    return keypair.publicKey;
}
