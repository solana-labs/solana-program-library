import {
    Connection,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    Transaction,
    TransactionSignature,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createBurnCheckedInstruction } from '../instructions';

/**
 * Burn tokens, asserting the token mint and decimals
 *
 * @param account      Account to burn tokens from
 * @param owner        Account owner
 * @param multiSigners Signing accounts if `owner` is a multiSig
 * @param amount       Amount to burn
 * @param decimals     Number of decimals in amount to burn
 *
 * @return Signature of the confirmed transaction
 */
export async function burnChecked(
    connection: Connection,
    mint: PublicKey,
    account: PublicKey,
    payer: Signer,
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
        createBurnCheckedInstruction(mint, account, ownerPublicKey, multiSigners, amount, decimals, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}
