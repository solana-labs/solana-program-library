import { AccountMeta, PublicKey, Signer } from '@solana/web3.js';

/** @internal */
export function addSigners(keys: AccountMeta[], pubkey: PublicKey, multiSigners: Signer[]): AccountMeta[] {
    if (multiSigners.length) {
        keys.push({ pubkey, isSigner: false, isWritable: false });
        for (const signer of multiSigners) {
            keys.push({ pubkey: signer.publicKey, isSigner: true, isWritable: false });
        }
    } else {
        keys.push({ pubkey, isSigner: true, isWritable: false });
    }
    return keys;
}
