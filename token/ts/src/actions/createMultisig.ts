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
import { createInitializeMultisigInstruction } from '../instructions';
import { getMinimumBalanceForRentExemptMultisig, MULTISIG_SIZE } from '../state';

/**
 * Create and initialize a new multisig
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction and initialization fees
 * @param signers        Full set of signers
 * @param m              Number of required signatures
 * @param confirmOptions Options for confirming the transaction
 * @param programId      SPL Token program account
 *
 * @return Address of the new multisig
 */
export async function createMultisig(
    connection: Connection,
    payer: Signer,
    signers: PublicKey[],
    m: number,
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_PROGRAM_ID
): Promise<PublicKey> {
    const lamports = await getMinimumBalanceForRentExemptMultisig(connection);

    const multisig = Keypair.generate();

    const transaction = new Transaction().add(
        SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: multisig.publicKey,
            space: MULTISIG_SIZE,
            lamports,
            programId,
        }),
        createInitializeMultisigInstruction(multisig.publicKey, signers, m, programId)
    );

    await sendAndConfirmTransaction(connection, transaction, [payer, multisig], confirmOptions);

    return multisig.publicKey;
}
