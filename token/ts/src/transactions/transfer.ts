import {
    Connection,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    Transaction,
    TransactionSignature,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createTransferInstruction } from '../instructions';

/**
 * Transfer tokens to another account
 *
 * @param source       Source account
 * @param destination  Destination account
 * @param owner        Owner of the source account
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param amount       Number of tokens to transfer
 *
 * @return Signature of the confirmed transaction
 */
export async function transfer(
    connection: Connection,
    payer: Signer,
    source: PublicKey,
    destination: PublicKey,
    owner: Signer | PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let ownerPublicKey: PublicKey;
    let signers: Signer[];
    if (owner instanceof PublicKey) {
        ownerPublicKey = owner;
        signers = multiSigners;
    } else {
        ownerPublicKey = owner.publicKey;
        signers = [owner];
    }

    const transaction = new Transaction().add(
        createTransferInstruction(source, destination, ownerPublicKey, multiSigners, amount, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}
