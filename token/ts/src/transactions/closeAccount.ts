import {
    Connection,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    Transaction,
    TransactionSignature,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createCloseAccountInstruction } from '../instructions';

/**
 * Close account
 *
 * @param account      Account to close
 * @param dest         Account to receive the remaining balance of the closed account
 * @param authority    Authority which is allowed to close the account
 * @param multiSigners Signing accounts if `authority` is a multiSig
 *
 * @return Signature of the confirmed transaction
 */
export async function closeAccount(
    connection: Connection,
    payer: Signer,
    account: PublicKey,
    dest: PublicKey,
    authority: Signer | PublicKey,
    multiSigners: Signer[],
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let authorityPublicKey;
    let signers: Signer[];
    if (authority instanceof PublicKey) {
        authorityPublicKey = authority;
        signers = multiSigners;
    } else {
        authorityPublicKey = authority.publicKey;
        signers = [authority];
    }

    const transaction = new Transaction().add(
        createCloseAccountInstruction(account, dest, authorityPublicKey, multiSigners, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}
