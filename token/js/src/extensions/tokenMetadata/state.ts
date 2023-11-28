import type { Commitment, Connection } from '@solana/web3.js';
import { PublicKey } from '@solana/web3.js';
import type { TokenMetadata } from '@solana/spl-token-metadata';
import { Field, unpack } from '@solana/spl-token-metadata';

import { TOKEN_2022_PROGRAM_ID } from '../../constants.js';
import { ExtensionType, getExtensionData } from '../extensionType.js';
import { getMint } from '../../state/mint.js';

export const getNormalizedTokenMetadataField = (field: Field | string): string => {
    if (field === Field.Name || field === 'Name' || field === 'name') {
        return 'name';
    }

    if (field === Field.Symbol || field === 'Symbol' || field === 'symbol') {
        return 'symbol';
    }

    if (field === Field.Uri || field === 'Uri' || field === 'uri') {
        return 'uri';
    }

    return field;
};

export const updateTokenMetadata = (
    current: TokenMetadata,
    key: Field | string,
    value: string | PublicKey | null
): TokenMetadata => {
    const field = getNormalizedTokenMetadataField(key);

    if (field === 'mint' && !(value instanceof PublicKey)) {
        throw new Error('TokenMetadata field mint must be a PublicKey');
    }

    if (field === 'updateAuthority' && !(value === null || value instanceof PublicKey)) {
        throw new Error('TokenMetadata field updateAuthority must be a PublicKey or null');
    }

    if (typeof value !== 'string' && field !== 'updateAuthority' && field !== 'mint') {
        throw new Error('TokenMetadata value must be a string');
    }

    // Handle case where updateAuthority is being removed
    if (field === 'updateAuthority' && value === null) {
        const { updateAuthority, ...state } = current;
        return state;
    }

    // Handle updates to default keys
    if (['mint', 'name', 'symbol', 'updateAuthority', 'uri'].includes(field)) {
        return {
            ...current,
            [field]: value,
        };
    }

    // If we are here, we are updating the additional metadata
    value = value as string; // Should already be enforced by above checks

    // Avoid mutating input, make a shallow copy
    const additionalMetadata = [...current.additionalMetadata];

    const i = current.additionalMetadata.findIndex((x) => x[0] === field);

    if (i === -1) {
        // Key was not found, add it
        additionalMetadata.push([field, value]);
    } else {
        // Key was found, change value
        additionalMetadata[i] = [field, value];
    }

    return {
        ...current,
        additionalMetadata,
    };
};

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
