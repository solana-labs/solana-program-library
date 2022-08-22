import { ConfirmOptions, Connection, sendAndConfirmTransaction, Signer, Transaction } from '@solana/web3.js';
import { TOKEN_2022_PROGRAM_ID, NATIVE_MINT_2022 } from '../constants.js';
import { createCreateNativeMintInstruction } from '../instructions/index.js';

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
    nativeMint = NATIVE_MINT_2022,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<void> {
    const transaction = new Transaction().add(
        createCreateNativeMintInstruction(payer.publicKey, nativeMint, programId)
    );
    await sendAndConfirmTransaction(connection, transaction, [payer], confirmOptions);
}
