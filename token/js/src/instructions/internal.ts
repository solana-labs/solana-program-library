import type { AccountMeta, Signer } from '@solana/web3.js';
import { PublicKey, Keypair } from '@solana/web3.js';

/** @internal */
export function addSigners(
    keys: AccountMeta[],
    ownerOrAuthority: PublicKey,
    multiSigners: (Signer | PublicKey)[]
): AccountMeta[] {
    if (multiSigners.length) {
        keys.push({ pubkey: ownerOrAuthority, isSigner: false, isWritable: false });
        
        for (const signer of multiSigners) {
            if (signer instanceof PublicKey || signer instanceof Keypair) {
                keys.push({
                    pubkey: 'publicKey' in signer ? signer.publicKey : signer,
                    isSigner: true,
                    isWritable: false,
                });
            }
            else if (signer.toString()) {
                try {
                    const compatiblePubkey = new PublicKey(signer.toString())
                    keys.push({
                        pubkey: compatiblePubkey,
                        isSigner: true,
                        isWritable: false,
                    });
                }
                catch (e) {
                     // not a pubkey 
                }
            }
        }
    } else {
        keys.push({ pubkey: ownerOrAuthority, isSigner: true, isWritable: false });
    }
    return keys;
}
