import { struct } from '@solana/buffer-layout';
import type { Mint } from '../state/mint.js';
import { ExtensionType, getExtensionData } from './extensionType.js';

/** Non-transferable state as stored by the program */
export interface NonTransferable {} // eslint-disable-line

/** Buffer layout for de/serializing an account */
export const NonTransferableLayout = struct<NonTransferable>([]);

export const NON_TRANSFERABLE_SIZE = NonTransferableLayout.span;

export function getNonTransferable(mint: Mint): NonTransferable | null {
    const extensionData = getExtensionData(ExtensionType.NonTransferable, mint.tlvData);
    if (extensionData !== null) {
        return NonTransferableLayout.decode(extensionData);
    } else {
        return null;
    }
}
