import { struct } from '@solana/buffer-layout';
import type { Mint } from '../../state/mint.js';
import { ExtensionType, getExtensionData } from '../extensionType.js';
import type { PublicKey } from '@solana/web3.js';
import { publicKey } from '@solana/buffer-layout-utils';

/** TransferHook as stored by the program */
export interface TransferHook {
    /** The transfer hook update authrority */
    authority: PublicKey;
    /** The transfer hook program account */
    programId: PublicKey;
}

/** Buffer layout for de/serializing a transfer fee config extension */
export const TransferHookLayout = struct<TransferHook>([publicKey('authority'), publicKey('programId')]);

export const TRANSFER_HOOK_SIZE = TransferHookLayout.span;

export function getTransferHook(mint: Mint): TransferHook | null {
    const extensionData = getExtensionData(ExtensionType.TransferHook, mint.tlvData);
    if (extensionData !== null) {
        return TransferHookLayout.decode(extensionData);
    } else {
        return null;
    }
}
