import {
    Connection,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    Transaction,
    TransactionSignature,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createTransferCheckedInstruction } from '../instructions';

/**
 * Transfer tokens to another account, asserting the token mint and decimals
 *
 * @param source       Source account
 * @param destination  Destination account
 * @param owner        Owner of the source account
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param amount       Number of tokens to transfer
 * @param decimals     Number of decimals in transfer amount
 *
 * @return Signature of the confirmed transaction
 */
export async function transferChecked(
    connection: Connection,
    mint: PublicKey,
    payer: Signer,
    source: PublicKey,
    destination: PublicKey,
    owner: Signer | PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    decimals: number,
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
        createTransferCheckedInstruction(
            source,
            mint,
            destination,
            ownerPublicKey,
            multiSigners,
            amount,
            decimals,
            programId
        )
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}
