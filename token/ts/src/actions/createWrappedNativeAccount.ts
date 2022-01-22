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
import { ASSOCIATED_TOKEN_PROGRAM_ID, NATIVE_MINT, TOKEN_PROGRAM_ID } from '../constants';
import {
    createAssociatedTokenAccountInstruction,
    createInitializeAccountInstruction,
    createSyncNativeInstruction,
} from '../instructions/index';
import { ACCOUNT_SIZE, getAssociatedTokenAddress, getMinimumBalanceForRentExemptAccount } from '../state/index';
import { createAccount } from './createAccount';

/**
 * Create, initialize, and fund a new wrapped native SOL account
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction and initialization fees
 * @param owner          Owner of the new token account
 * @param amount         Number of lamports to wrap
 * @param keypair        Optional keypair, defaulting to the associated token account for the native mint and `owner`
 * @param confirmOptions Options for confirming the transaction
 * @param programId      SPL Token program account
 *
 * @return Address of the new wrapped native SOL account
 */
export async function createWrappedNativeAccount(
    connection: Connection,
    payer: Signer,
    owner: PublicKey,
    amount: number,
    keypair?: Keypair,
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_PROGRAM_ID
): Promise<PublicKey> {
    // If the amount provided is explicitly 0 or NaN, just create the account without funding it
    if (!amount) return await createAccount(connection, payer, NATIVE_MINT, owner, keypair, confirmOptions, programId);

    // If a keypair isn't provided, create the account at the owner's ATA for the native mint and return its address
    if (!keypair) {
        const associatedToken = await getAssociatedTokenAddress(
            NATIVE_MINT,
            owner,
            false,
            programId,
            ASSOCIATED_TOKEN_PROGRAM_ID
        );

        const transaction = new Transaction().add(
            createAssociatedTokenAccountInstruction(
                payer.publicKey,
                associatedToken,
                owner,
                NATIVE_MINT,
                programId,
                ASSOCIATED_TOKEN_PROGRAM_ID
            ),
            SystemProgram.transfer({
                fromPubkey: payer.publicKey,
                toPubkey: associatedToken,
                lamports: amount,
            }),
            createSyncNativeInstruction(associatedToken, programId)
        );

        await sendAndConfirmTransaction(connection, transaction, [payer], confirmOptions);

        return associatedToken;
    }

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
        SystemProgram.transfer({
            fromPubkey: payer.publicKey,
            toPubkey: keypair.publicKey,
            lamports: amount,
        }),
        createInitializeAccountInstruction(keypair.publicKey, NATIVE_MINT, owner, programId)
    );

    await sendAndConfirmTransaction(connection, transaction, [payer, keypair], confirmOptions);

    return keypair.publicKey;
}
