import type { ConfirmOptions, Connection, PublicKey, Signer, TransactionSignature } from '@solana/web3.js';
import type { Field } from '@solana/spl-token-metadata';
import {
    createInitializeInstruction,
    createEmitInstruction,
    createUpdateFieldInstruction,
} from '@solana/spl-token-metadata';
import { sendAndConfirmTransaction, Transaction } from '@solana/web3.js';

import { TOKEN_2022_PROGRAM_ID } from '../../constants.js';
import { getSigners } from '../../actions/internal.js';

/**
 * Initializes a TLV entry with the basic token-metadata fields.
 *
 * @param connection       Connection to use
 * @param payer            Payer of the transaction fees
 * @param updateAuthority  Update Authority
 * @param mint             Mint Account
 * @param mintAuthority    Mint Authority
 * @param name             Longer name of token
 * @param symbol           Shortened symbol of token
 * @param uri              URI pointing to more metadata (image, video, etc)
 * @param multiSigners     Signing accounts if `authority` is a multisig
 * @param confirmOptions   Options for confirming the transaction
 * @param programId        SPL Token program account
 *
 * @return Signature of the confirmed transaction
 */
export async function tokenMetadataInitialize(
    connection: Connection,
    payer: Signer,
    updateAuthority: PublicKey,
    mint: PublicKey,
    mintAuthority: PublicKey | Signer,
    name: string,
    symbol: string,
    uri: string,
    multiSigners: Signer[] = [],
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<TransactionSignature> {
    const [mintAuthorityPublicKey, signers] = getSigners(mintAuthority, multiSigners);

    const transaction = new Transaction().add(
        createInitializeInstruction({
            programId,
            metadata: mint,
            updateAuthority,
            mint,
            mintAuthority: mintAuthorityPublicKey,
            name,
            symbol,
            uri,
        })
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers], confirmOptions);
}

/**
 * Updates a field in a token-metadata account.
 * If the field does not exist on the account, it will be created.
 * If the field does exist, it will be overwritten.
 *
 * The field can be one of the required fields (name, symbol, URI), or a
 * totally new field denoted by a "key" string.
 * @param connection       Connection to use
 * @param payer            Payer of the transaction fees
 * @param updateAuthority  Update Authority
 * @param mint             Mint Account
 * @param field            Longer name of token
 * @param value            Shortened symbol of token
 * @param multiSigners     Signing accounts if `authority` is a multisig
 * @param confirmOptions   Options for confirming the transaction
 * @param programId        SPL Token program account
 *
 * @return Signature of the confirmed transaction
 */
export async function tokenMetadataUpdateField(
    connection: Connection,
    payer: Signer,
    updateAuthority: PublicKey | Signer,
    mint: PublicKey,
    field: string | Field,
    value: string,
    multiSigners: Signer[] = [],
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<TransactionSignature> {
    const [updateAuthorityPublicKey, signers] = getSigners(updateAuthority, multiSigners);

    const transaction = new Transaction().add(
        createUpdateFieldInstruction({
            programId,
            metadata: mint,
            updateAuthority: updateAuthorityPublicKey,
            field,
            value,
        })
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers], confirmOptions);
}

/**
 * Emits the token-metadata as return data
 *
 * @param connection       Connection to use
 * @param payer            Payer of the transaction fees
 * @param mint             Mint Account
 * @param multiSigners     Signing accounts if `authority` is a multisig
 * @param confirmOptions   Options for confirming the transaction
 * @param programId        SPL Token program account
 *
 * @return Signature of the confirmed transaction
 */
export async function tokenMetadataEmit(
    connection: Connection,
    payer: Signer,
    mint: PublicKey,
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<TransactionSignature> {
    const transaction = new Transaction().add(
        createEmitInstruction({
            programId,
            metadata: mint,
        })
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer], confirmOptions);
}
