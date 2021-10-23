import { struct, u8 } from '@solana/buffer-layout';
import { publicKey } from '@solana/buffer-layout-utils';
import { PublicKey } from '@solana/web3.js';

/**
 * Information about a multisig
 */
export interface Multisig {
    /**
     * The number of signers required
     */
    m: number;

    /**
     * Number of possible signers, corresponds to the
     * number of `signers` that are valid.
     */
    n: number;

    /**
     * Is this mint initialized
     */
    isInitialized: boolean;

    /**
     * The signers
     */
    signer1: PublicKey;
    signer2: PublicKey;
    signer3: PublicKey;
    signer4: PublicKey;
    signer5: PublicKey;
    signer6: PublicKey;
    signer7: PublicKey;
    signer8: PublicKey;
    signer9: PublicKey;
    signer10: PublicKey;
    signer11: PublicKey;
}

export const MultisigLayout = struct<Multisig>([
    u8('m'),
    u8('n'),
    u8('isInitialized'),
    publicKey('signer1'),
    publicKey('signer2'),
    publicKey('signer3'),
    publicKey('signer4'),
    publicKey('signer5'),
    publicKey('signer6'),
    publicKey('signer7'),
    publicKey('signer8'),
    publicKey('signer9'),
    publicKey('signer10'),
    publicKey('signer11'),
]);

export const MULTISIG_LEN = MultisigLayout.span;
