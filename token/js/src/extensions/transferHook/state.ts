import { struct } from '@solana/buffer-layout';
import type { Mint } from '../../state/mint.js';
import { ExtensionType, getExtensionData } from '../extensionType.js';
import type { PublicKey } from '@solana/web3.js';
import { bool, publicKey } from '@solana/buffer-layout-utils';
import type { Account } from '../../state/account.js';

/** TransferHook as stored by the program */
export interface TransferHook {
    /** The transfer hook update authrority */
    authority: PublicKey;
    /** The transfer hook program account */
    programId: PublicKey;
}

/** Buffer layout for de/serializing a transfer hook extension */
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

/** TransferHookAccount as stored by the program */
export interface TransferHookAccount {
    /**
     * Whether or not this account is currently tranferring tokens
     * True during the transfer hook cpi, otherwise false
     */
    transferring: boolean;
}

/** Buffer layout for de/serializing a transfer hook account extension */
export const TransferHookAccountLayout = struct<TransferHookAccount>([bool('transferring')]);

export const TRANSFER_HOOK_ACCOUNT_SIZE = TransferHookAccountLayout.span;

export function getTransferHookAccount(account: Account): TransferHookAccount | null {
    const extensionData = getExtensionData(ExtensionType.TransferHookAccount, account.tlvData);
    if (extensionData !== null) {
        return TransferHookAccountLayout.decode(extensionData);
    } else {
        return null;
    }
}
