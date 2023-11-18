import type { Commitment, Connection, Finality, PublicKey } from '@solana/web3.js';
import type { TokenMetadata } from '@solana/spl-token-metadata';
import { unpack } from '@solana/spl-token-metadata';

import { TOKEN_2022_PROGRAM_ID } from '../../constants.js';
import { ExtensionType, getExtensionData } from '../extensionType.js';
import { getMint } from '../../state/mint.js';

/**
 * Retrieve Token Metadata Information
 *
 * @param connection Connection to use
 * @param address    Mint account
 * @param commitment Desired level of commitment for querying the state
 * @param programId  SPL Token program account
 *
 * @return Token Metadata information
 */
export async function getTokenMetadata(
    connection: Connection,
    address: PublicKey,
    commitment?: Commitment,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<TokenMetadata | null> {
    const mintInfo = await getMint(connection, address, commitment, programId);
    const data = getExtensionData(ExtensionType.TokenMetadata, mintInfo.tlvData);

    if (data === null) {
        return null;
    }

    return unpack(data);
}

/**
 * Retrieve Token Metadata Information emitted in transaction
 *
 * @param connection Connection to use
 * @param signature  Transaction signature
 * @param commitment Desired level of commitment for querying the state
 *
 * @return Token Metadata information
 */
export async function getEmittedTokenMetadata(
    connection: Connection,
    signature: string,
    commitment?: Finality,
    programId = TOKEN_2022_PROGRAM_ID
): Promise<TokenMetadata | null> {
    const tx: any = await connection.getTransaction(signature, {
        commitment: commitment,
        maxSupportedTransactionVersion: 2,
    });

    if (tx === null) {
        return null;
    }

    const data = Buffer.from(tx?.meta?.returnData?.data?.[0], 'base64');

    if (data === null) {
        return null;
    }

    return unpack(data);
}
