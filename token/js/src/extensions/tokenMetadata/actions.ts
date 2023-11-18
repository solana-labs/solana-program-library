import type { ConfirmOptions, Connection, PublicKey, Signer, TransactionSignature } from '@solana/web3.js';
import type { Field, TokenMetadata } from '@solana/spl-token-metadata';
import {
    createEmitInstruction,
    createInitializeInstruction,
    createRemoveKeyInstruction,
    createUpdateAuthorityInstruction,
    createUpdateFieldInstruction,
    pack,
} from '@solana/spl-token-metadata';
import { sendAndConfirmTransaction, SystemProgram, Transaction } from '@solana/web3.js';
import { TokenAccountNotFoundError } from '../../errors.js';

import { TOKEN_2022_PROGRAM_ID } from '../../constants.js';
import { getSigners } from '../../actions/internal.js';
import { unpackMint } from '../../state/mint.js';
import { getExtensionData, ExtensionType } from '../extensionType.js';

/**
 * Calculates additional lamports need to variable length extension
 *
 * @param connection       Connection to use
 * @param address          Mint Account
 * @param tokenMetadata    Token Metadata
 * @param programId        SPL Token program account
 *
 * @return lamports to send
 */
async function getAdditionalRentForNewMetadata(
    connection: Connection,
    address: PublicKey,
    tokenMetadata: TokenMetadata,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<number> {
    const info = await connection.getAccountInfo(address);
    const mint = unpackMint(address, info, programId);

    if (!info) {
        // unpack mint does error checking on info internally
        // but just for typescript and just incase
        throw new TokenAccountNotFoundError();
    }

    const data = getExtensionData(ExtensionType.TokenMetadata, mint.tlvData);

    const currentDataLen = data ? data.length : 0;
    const newDataLen = pack(tokenMetadata).length;

    let newAccountLen = info.data.length + (newDataLen - currentDataLen);

    if (currentDataLen === 0) {
        // Extension not initialized
        // Need 2 bytes extension discriminator, 2 bytes length
        newAccountLen += 4;
    }

    const newRentExemptMinimum = await connection.getMinimumBalanceForRentExemption(newAccountLen);

    return newRentExemptMinimum - info.lamports;
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

    const lamports = await getAdditionalRentForNewMetadata(connection, mint, {
        updateAuthority,
        mint,
        name,
        symbol,
        uri,
        additionalMetadata: [],
    });

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
 * @param field            Longer name of token
 * @param value            Shortened symbol of token
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

    // TODO:- Figure this out
    // const lamports = await getAdditionalRentForNewMetadata(connection, mint, {
    //     updateAuthority,
    //     mint,
    //     name,
    //     symbol,
    //     uri,
    //     additionalMetadata: [],
    // });
    const lamports = 0; // TODO

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
