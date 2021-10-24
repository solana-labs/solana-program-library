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
import { createInitializeAccountInstruction } from '../instructions';
import { ACCOUNT_SIZE, getMinimumBalanceForRentExemptAccount } from '../state';

/**
 * Create and initialize a new token account
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction and initialization fees
 * @param mint           Mint for the account
 * @param owner          Owner of the new account
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
        createInitializeAccountInstruction(account.publicKey, mint, owner, programId)
    );

    await sendAndConfirmTransaction(connection, transaction, [payer, account], confirmOptions);

    return account.publicKey;
}
