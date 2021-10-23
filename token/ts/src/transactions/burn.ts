import {
    Connection,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    Transaction,
    TransactionSignature,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createBurnInstruction } from '../instructions';

/**
 * Burn tokens
 *
 * @param account      Account to burn tokens from
 * @param owner        Account owner
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param amount       Amount to burn
 *
 * @return Signature of the confirmed transaction
 */
export async function burn(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    account: PublicKey,
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
        createBurnInstruction(mint, account, ownerPublicKey, multiSigners, amount, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}
