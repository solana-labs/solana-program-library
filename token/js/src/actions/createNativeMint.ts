import { ConfirmOptions, Connection, sendAndConfirmTransaction, Signer, Transaction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, NATIVE_MINT } from '../constants';
import { createCreateNativeMintInstruction } from '../instructions/index';

/**
 * Create native mint
 *
 * @param connection               Connection to use
 * @param payer                    Payer of the transaction and initialization fees
 * @param confirmOptions           Options for confirming the transaction
 * @param programId                SPL Token program account
 * @param nativeMint               Native mint id associated with program
 */
export async function createNativeMint(
    connection: Connection,
    payer: Signer,
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_PROGRAM_ID,
    nativeMint = NATIVE_MINT
): Promise<void> {
    const transaction = new Transaction().add(
        createCreateNativeMintInstruction(payer.publicKey, programId, nativeMint)
    );
    await sendAndConfirmTransaction(connection, transaction, [payer], confirmOptions);
}
