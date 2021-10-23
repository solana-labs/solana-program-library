import {
    Connection,
    PublicKey,
    sendAndConfirmTransaction,
    Signer,
    Transaction,
    TransactionSignature,
} from '@solana/web3.js';
import { TOKEN_PROGRAM_ID } from '../constants';
import { createMintToInstruction } from '../instructions';

/**
 * Mint new tokens
 *
 * @param dest         Public key of the account to mint to
 * @param authority    Minting authority
 * @param multiSigners Signing accounts if `authority` is a multiSig
 * @param amount       Amount to mint
 *
 * @return Signature of the confirmed transaction
 */
export async function mintTo(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    dest: PublicKey,
    authority: Signer | PublicKey,
    multiSigners: Signer[],
    amount: number | bigint,
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
        createMintToInstruction(mint, dest, authorityPublicKey, multiSigners, amount, programId)
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers]);
}
