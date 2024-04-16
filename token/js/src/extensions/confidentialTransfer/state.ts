import { struct } from '@solana/buffer-layout';
import { publicKey, bool } from '@solana/buffer-layout-utils';
import type { PublicKey } from '@solana/web3.js';
import type { Mint } from '../../state/mint.js';
import { ExtensionType, getExtensionData } from '../extensionType.js';
import { PodElGamalPubkey } from 'solana-zk-token-sdk-experimental';
import { elgamalPublicKey } from './elgamal.js';

export interface ConfidentialTransferMint {
    confidentialTransferMintAuthority: PublicKey;
    autoApproveNewAccounts: boolean;
    auditorElGamalPubkey: PodElGamalPubkey;
}

export const ConfidentialTransferMintLayout = struct<ConfidentialTransferMint>([
    publicKey('confidentialTransferMintAuthority'),
    bool('autoApproveNewAccounts'),
    elgamalPublicKey('auditorElGamalPubkey'),
]);

export const CONFIDENTIAL_TRANSFER_MINT_SIZE = ConfidentialTransferMintLayout.span;

export function getConfidentialTransferMint(mint: Mint): ConfidentialTransferMint | null {
    const extensionData = getExtensionData(ExtensionType.ConfidentialTransferMint, mint.tlvData);
    if (extensionData !== null) {
        return ConfidentialTransferMintLayout.decode(extensionData);
    } else {
        return null;
    }
}
