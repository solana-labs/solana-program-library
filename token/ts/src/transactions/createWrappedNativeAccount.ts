import {
    Connection,
    Keypair,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    SystemProgram,
    Transaction,
} from '@solana/web3.js';
import { NATIVE_MINT, TOKEN_PROGRAM_ID } from '../constants';
import { createInitializeAccountInstruction } from '../instructions';
import { ACCOUNT_LEN, getMinimumBalanceForRentExemptAccount } from '../state';

/**
 * Create and initialize a new wrapped native SOL account
 *
 * In order to be wrapped, the account must have a balance of native tokens
 * when it is initialized with the token program.
 *
 * This function sends lamports to the new account before initializing it
 *
 * @param connection A solana web3 connection
 * @param owner      The owner of the new token account
 * @param payer      The source of the lamports to initialize, and payer of the initialization fees
 * @param amount     The amount of lamports to wrap
 * @param programId  The token program ID
 *
 * @return Address of the new wrapped native SOL account
 */
export async function createWrappedNativeAccount(
    connection: Connection,
    owner: PublicKey,
    payer: Signer,
    amount: number,
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
        SystemProgram.transfer({
            fromPubkey: payer.publicKey,
            toPubkey: newAccount.publicKey,
            lamports: amount,
        }),
        createInitializeAccountInstruction(NATIVE_MINT, newAccount.publicKey, owner, programId)
    );

    await sendAndConfirmTransaction(connection, transaction, [payer, newAccount]);

    return newAccount.publicKey;
}
