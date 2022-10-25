import type { ConfirmOptions, Connection, Signer, TransactionSignature } from '@solana/web3.js';
import { PublicKey } from '@solana/web3.js';
import { sendAndConfirmTransaction, Transaction, TransactionInstruction } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, MEMO_PROGRAM_ID } from '../constants.js';
import { createTransferInstruction } from '../instructions/transfer.js';
import { getSigners } from './internal.js';

/**
 * Transfer tokens from one account to another
 *
 * @param connection     Connection to use
 * @param payer          Payer of the transaction fees
 * @param source         Source account
 * @param destination    Destination account
 * @param owner          Owner of the source account
 * @param amount         Number of tokens to transfer
 * @param multiSigners   Signing accounts if `owner` is a multisig
 * @param confirmOptions Options for confirming the transaction
 * @param programId      SPL Token program account
 * @param memoProgramId  Solana Memo program account
 * @param memo           text describing purpose of the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function transfer(
    connection: Connection,
    payer: Signer,
    source: PublicKey,
    destination: PublicKey,
    owner: Signer | PublicKey,
    amount: number | bigint,
    multiSigners: Signer[] = [],
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_PROGRAM_ID,
    memoProgramId = MEMO_PROGRAM_ID,
    memo?: string
): Promise<TransactionSignature> {
    const [ownerPublicKey, signers] = getSigners(owner, multiSigners);

    const transaction = new Transaction().add(
        createTransferInstruction(source, destination, ownerPublicKey, amount, multiSigners, programId)
    );

    // Add an (optional) note describing the transaction.
    // Don't bother adding if memo is an empty string
    if (memo?.length && memoProgramId) {
        await transaction.add(
            new TransactionInstruction({
                keys: [{ pubkey: source, isSigner: true, isWritable: true }],
                data: Buffer.from(memo, 'utf-8'),
                programId: new PublicKey(memoProgramId),
            })
        );
    }

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers], confirmOptions);
}
