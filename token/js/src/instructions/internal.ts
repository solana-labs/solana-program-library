import type { AccountMeta, PublicKey, Signer } from '@solana/web3.js';

/** @internal */
export function addSigners(keys: AccountMeta[], ownerOrAuthority: PublicKey, multiSigners: Signer[]): AccountMeta[] {
    if (multiSigners.length) {
        keys.push({ pubkey: ownerOrAuthority, isSigner: false, isWritable: false });
        for (const signer of multiSigners) {
            keys.push({ pubkey: signer.publicKey, isSigner: true, isWritable: false });
        }
    } else {
        keys.push({ pubkey: ownerOrAuthority, isSigner: true, isWritable: false });
    }
    return keys;
}
