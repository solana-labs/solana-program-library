import {
    Connection,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    Transaction,
    TransactionSignature,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createSyncNativeInstruction } from '../instructions';

/**
 * Sync amount in native SPL token account to underlying lamports
 *
 * @param nativeAccount Account to sync
 *
 * @return Signature of the confirmed transaction
 */
export async function syncNative(
    connection: Connection,
    nativeAccount: PublicKey,
    payer: Signer,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    const transaction = new Transaction().add(createSyncNativeInstruction(nativeAccount, programId));

    return await sendAndConfirmTransaction(connection, transaction, [payer]);
}
