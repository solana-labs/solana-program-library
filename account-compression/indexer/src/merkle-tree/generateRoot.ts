import { PublicKey } from '@solana/web3.js';
import { Proof } from './types';
import { hash } from './hash';
import * as bs58 from 'bs58';

export function generateRoot(proof: Proof) {
    let node = bs58.decode(proof.leaf);
    let index = proof.index;
    for (const [i, pNode] of proof.proofNodes.entries()) {
        if ((index >> i) % 2 === 0) {
            node = hash(node, new PublicKey(pNode).toBuffer());
        } else {
            node = hash(new PublicKey(pNode).toBuffer(), node);
        }
    }
    return node;
}