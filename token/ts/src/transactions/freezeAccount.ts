import {
    Connection,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    Transaction,
    TransactionSignature,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createFreezeAccountInstruction } from '../instructions';

/**
 * Freeze account
 *
 * @param account      Account to freeze
 * @param authority    The mint freeze authority
 * @param multiSigners Signing accounts if `authority` is a multiSig
 *
 * @return Signature of the confirmed transaction
 */
export async function freezeAccount(
    connection: Connection,
    mint: PublicKey,
    payer: Signer,
    account: PublicKey,
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
        createFreezeAccountInstruction(account, mint, authorityPublicKey, multiSigners, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}
