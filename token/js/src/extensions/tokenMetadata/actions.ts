import type { ConfirmOptions, Connection, PublicKey, Signer, TransactionSignature } from '@solana/web3.js';
import { sendAndConfirmTransaction, SystemProgram, Transaction } from '@solana/web3.js';
import type { Field, TokenMetadata } from '@solana/spl-token-metadata';
import {
    createEmitInstruction,
    createInitializeInstruction,
    createRemoveKeyInstruction,
    createUpdateAuthorityInstruction,
    createUpdateFieldInstruction,
    pack,
    unpack,
} from '@solana/spl-token-metadata';

import { TOKEN_2022_PROGRAM_ID } from '../../constants.js';
import { getSigners } from '../../actions/internal.js';
import {
    ExtensionType,
    getAdditionalRentForNewExtensionLen,
    getExtensionDataFromAccountInfo,
} from '../extensionType.js';
import { updateTokenMetadata } from './state.js';

async function getAdditionalRentForNewMetadata(
    connection: Connection,
    address: PublicKey,
    tokenMetadata: TokenMetadata,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<number> {
    return await getAdditionalRentForNewExtensionLen(
        connection,
        address,
        ExtensionType.TokenMetadata,
        pack(tokenMetadata).length,
        programId
    );
}

async function getAdditionalRentForUpdatedMetadata(
    connection: Connection,
    address: PublicKey,
    field: string | Field,
    value: string,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<number> {
    const info = await connection.getAccountInfo(address);

    const data = getExtensionDataFromAccountInfo(address, info, ExtensionType.TokenMetadata, programId);

    if (data === null) {
        throw new Error('TokenMetadata extension not initialised');
    }

    const updatedTokenMetadata = updateTokenMetadata(unpack(data), field, value);

    const extensionLen = pack(updatedTokenMetadata).length;

    return await getAdditionalRentForNewExtensionLen(
        connection,
        address,
        ExtensionType.TokenMetadata,
        extensionLen,
        programId,
        info
    );
}

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
 * Initializes a TLV entry with the basic token-metadata fields,
 * Includes a transfer for any additional rent-exempt SOL if required.
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
export async function tokenMetadataInitializeWithRentTransfer(
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

    const transaction = new Transaction();

    const lamports = await getAdditionalRentForNewMetadata(
        connection,
        mint,
        {
            updateAuthority,
            mint,
            name,
            symbol,
            uri,
            additionalMetadata: [],
        },
        programId
    );

    if (lamports > 0) {
        transaction.add(SystemProgram.transfer({ fromPubkey: payer.publicKey, toPubkey: mint, lamports: lamports }));
    }

    transaction.add(
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
 * @param field            Field to update in the metadata
 * @param value            Value to write for the field
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
 * Updates a field in a token-metadata account.
 * If the field does not exist on the account, it will be created.
 * If the field does exist, it will be overwritten.
 * Includes a transfer for any additional rent-exempt SOL if required.
 *
 * The field can be one of the required fields (name, symbol, URI), or a
 * totally new field denoted by a "key" string.
 * @param connection       Connection to use
 * @param payer            Payer of the transaction fees
 * @param updateAuthority  Update Authority
 * @param mint             Mint Account
 * @param field            Field to update in the metadata
 * @param value            Value to write for the field
 * @param multiSigners     Signing accounts if `authority` is a multisig
 * @param confirmOptions   Options for confirming the transaction
 * @param programId        SPL Token program account
 *
 * @return Signature of the confirmed transaction
 */
export async function tokenMetadataUpdateFieldWithRentTransfer(
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

    const transaction = new Transaction();

    const lamports = await getAdditionalRentForUpdatedMetadata(connection, mint, field, value, programId);

    if (lamports > 0) {
        transaction.add(SystemProgram.transfer({ fromPubkey: payer.publicKey, toPubkey: mint, lamports: lamports }));
    }

    transaction.add(
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

/**
 * Remove a field in a token-metadata account.
 *
 * The field can be one of the required fields (name, symbol, URI), or a
 * totally new field denoted by a "key" string.
 * @param connection       Connection to use
 * @param payer            Payer of the transaction fees
 * @param updateAuthority  Update Authority
 * @param mint             Mint Account
 * @param key              Key to remove in the additional metadata portion
 * @param idempotent       When true, instruction will not error if the key does not exist
 * @param multiSigners     Signing accounts if `authority` is a multisig
 * @param confirmOptions   Options for confirming the transaction
 * @param programId        SPL Token program account
 *
 * @return Signature of the confirmed transaction
 */
export async function tokenMetadataRemoveKey(
    connection: Connection,
    payer: Signer,
    updateAuthority: PublicKey | Signer,
    mint: PublicKey,
    key: string,
    idempotent: boolean,
    multiSigners: Signer[] = [],
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<TransactionSignature> {
    const [updateAuthorityPublicKey, signers] = getSigners(updateAuthority, multiSigners);

    const transaction = new Transaction().add(
        createRemoveKeyInstruction({
            programId,
            metadata: mint,
            updateAuthority: updateAuthorityPublicKey,
            key,
            idempotent,
        })
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers], confirmOptions);
}

/**
 *  Update authority
 *
 * @param connection       Connection to use
 * @param payer            Payer of the transaction fees
 * @param updateAuthority  Update Authority
 * @param mint             Mint Account
 * @param newAuthority     New authority for the token metadata, or unset
 * @param multiSigners     Signing accounts if `authority` is a multisig
 * @param confirmOptions   Options for confirming the transaction
 * @param programId        SPL Token program account
 *
 * @return Signature of the confirmed transaction
 */
export async function tokenMetadataUpdateAuthority(
    connection: Connection,
    payer: Signer,
    updateAuthority: PublicKey | Signer,
    mint: PublicKey,
    newAuthority: PublicKey | null,
    multiSigners: Signer[] = [],
    confirmOptions?: ConfirmOptions,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<TransactionSignature> {
    const [updateAuthorityPublicKey, signers] = getSigners(updateAuthority, multiSigners);

    const transaction = new Transaction().add(
        createUpdateAuthorityInstruction({
            programId,
            metadata: mint,
            oldAuthority: updateAuthorityPublicKey,
            newAuthority,
        })
    );

    return await sendAndConfirmTransaction(connection, transaction, [payer, ...signers], confirmOptions);
}
