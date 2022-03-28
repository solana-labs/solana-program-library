import { struct } from '@solana/buffer-layout';
import { publicKey } from '@solana/buffer-layout-utils';
import { PublicKey } from '@solana/web3.js';
import { Mint } from '../state/mint';
import { ExtensionType, getExtensionData } from './extensionType';

/** MintCloseAuthority as stored by the program */
export interface ImmutableOwner {
    owner: PublicKey;
}

/** Buffer layout for de/serializing a mint */
export const ImmutableOwnerLayout = struct<ImmutableOwner>([publicKey('owner')]);

export const IMMUTABLE_OWNER_SIZE = ImmutableOwnerLayout.span;

export function getImmutableOwner(mint: Mint): ImmutableOwner | null {
    const extensionData = getExtensionData(ExtensionType.ImmutableOwner, mint.tlvData);
    if (extensionData !== null) {
        return ImmutableOwnerLayout.decode(extensionData);
    } else {
        return null;
    }
}