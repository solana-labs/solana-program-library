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
import { NATIVE_MINT, TOKEN_PROGRAM_ID } from '../constants';
import { createInitializeAccountInstruction } from '../instructions';
import { ACCOUNT_SIZE, getMinimumBalanceForRentExemptAccount } from '../state';

/**
 * Create and initialize a new wrapped native SOL account
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction and initialization fees
 * @param owner          Owner of the new token account
 * @param amount         Number of lamports to wrap
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
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_PROGRAM_ID
): Promise<PublicKey> {
    const lamports = await getMinimumBalanceForRentExemptAccount(connection);

    const account = Keypair.generate();

    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: account.publicKey,
            space: ACCOUNT_SIZE,
            lamports,
            programId,
        }),
        SystemProgram.transfer({
            fromPubkey: payer.publicKey,
            toPubkey: account.publicKey,
            lamports: amount,
        }),
        createInitializeAccountInstruction(account.publicKey, NATIVE_MINT, owner, programId)
    );

    await sendAndConfirmTransaction(connection, transaction, [payer, account], confirmOptions);

    return account.publicKey;
}
