import {
    Connection,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    Transaction,
    TransactionSignature,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createMintToCheckedInstruction } from '../instructions';

/**
 * Mint new tokens, asserting the token mint and decimals
 *
 * @param dest         Public key of the account to mint to
 * @param authority    Minting authority
 * @param multiSigners Signing accounts if `authority` is a multiSig
 * @param amount       Amount to mint
 * @param decimals     Number of decimals in amount to mint
 *
 * @return Signature of the confirmed transaction
 */
export async function mintToChecked(
    connection: Connection,
    mint: PublicKey,
    payer: Signer,
    dest: PublicKey,
    authority: Signer | PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
    decimals: number,
    programId = TOKEN_PROGRAM_ID
): Promise<TransactionSignature> {
    let authorityPublicKey: PublicKey;
    let signers: Signer[];
    if (authority instanceof PublicKey) {
        authorityPublicKey = authority;
        signers = multiSigners;
    } else {
        authorityPublicKey = authority.publicKey;
        signers = [authority];
    }

    const transaction = new Transaction().add(
        createMintToCheckedInstruction(mint, dest, authorityPublicKey, multiSigners, amount, decimals, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}
