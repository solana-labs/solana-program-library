import { struct } from '@solana/buffer-layout';
import { publicKey } from '@solana/buffer-layout-utils';
import { PublicKey } from '@solana/web3.js';
import type { Mint } from '../../state/mint.js';
import { ExtensionType, getExtensionData } from '../extensionType.js';

/** MetadataPointer as stored by the program */
export interface MetadataPointer {
    /** Optional authority that can set the metadata address */
    authority?: PublicKey;
    /** Optional Account Address that holds the metadata */
    metadataAddress?: PublicKey;
}

/** Buffer layout for de/serializing a Metadata Pointer extension */
export const MetadataPointerLayout = struct<Required<MetadataPointer>>([
    publicKey('authority'),
    publicKey('metadataAddress'),
]);

export const METADATA_POINTER_SIZE = MetadataPointerLayout.span;

export function getMetadataPointerState(mint: Mint): Partial<MetadataPointer> | null {
    const extensionData = getExtensionData(ExtensionType.MetadataPointer, mint.tlvData);
    if (extensionData !== null) {
        const state: MetadataPointer = {};
        const decoded = MetadataPointerLayout.decode(extensionData);

        // Only add keys if they are defined (ignored zero public keys)
        if (!decoded.authority.equals(PublicKey.default)) {
            state.authority = decoded.authority;
        }
        if (!decoded.metadataAddress.equals(PublicKey.default)) {
            state.metadataAddress = decoded.metadataAddress;
        }

        return state;
    } else {
        return null;
    }
}
