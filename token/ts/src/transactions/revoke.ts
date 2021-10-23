import {
    Connection,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    Transaction,
    TransactionSignature,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createRevokeInstruction } from '../instructions';

/**
 * Remove approval for the transfer of any remaining tokens
 *
 * @param account      Public key of the account
 * @param owner        Owner of the source account
 * @param multiSigners Signing accounts if `owner` is a multiSig
 *
 * @return Signature of the confirmed transaction
 */
export async function revoke(
    connection: Connection,
    payer: Signer,
    account: PublicKey,
    owner: Signer | PublicKey,
    multiSigners: Signer[],
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
        createRevokeInstruction(account, ownerPublicKey, multiSigners, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}
