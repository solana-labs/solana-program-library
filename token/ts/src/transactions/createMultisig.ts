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
import { createInitializeMultisigInstruction } from '../instructions';
import { getMinimumBalanceForRentExemptMultisig, MULTISIG_LEN } from '../state';

/**
 * Create and initialize a new multisig
 *
 * @param connection   A solana web3 connection
 * @param payer        Payer of the initialization fees
 * @param m            Number of required signatures
 * @param multiSigners Full set of signers
 *
 * @return Address of the new multisig account
 */
export async function createMultisig(
    connection: Connection,
    payer: Signer,
    m: number,
    multiSigners: PublicKey[],
    programId = TOKEN_PROGRAM_ID
): Promise<PublicKey> {
    const lamports = await getMinimumBalanceForRentExemptMultisig(connection);

    const newAccount = Keypair.generate();

    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: newAccount.publicKey,
            space: MULTISIG_LEN,
            lamports,
            programId,
        }),
        createInitializeMultisigInstruction(newAccount.publicKey, multiSigners, m, programId)
    );

    await sendAndConfirmTransaction(connection, transaction, [payer, newAccount]);

    return newAccount.publicKey;
}
