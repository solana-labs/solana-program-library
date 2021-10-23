import { struct, u32, u8 } from '@solana/buffer-layout';
import { bool, publicKey, u64 } from '@solana/buffer-layout-utils';
import { PublicKey } from '@solana/web3.js';

/**
 * Information about the mint
 */
export interface Mint {
    /**
     * The address of this mint
     */
    address: PublicKey;

    /**
     * Optional authority used to mint new tokens. The mint authority may only be provided during
     * mint creation. If no mint authority is present then the mint has a fixed supply and no
     * further tokens may be minted.
     */
    mintAuthority: PublicKey | null;

    /**
     * Total supply of tokens
     */
    supply: bigint;

    /**
     * Number of base 10 digits to the right of the decimal place
     */
    decimals: number;

    /**
     * Is this mint initialized
     */
    isInitialized: boolean;

    /**
     * Optional authority to freeze token accounts
     */
    freezeAuthority: PublicKey | null;
}

export interface RawMint {
    mintAuthority: PublicKey | null;
    supply: bigint;
    decimals: number;
    isInitialized: boolean;
    freezeAuthority: PublicKey | null;
}

export const MintLayout = struct<RawMint>([
    u32('mintAuthorityOption'),
    publicKey('mintAuthority'),
    u64('supply'),
    u8('decimals'),
    bool('isInitialized'),
    u32('freezeAuthorityOption'),
    publicKey('freezeAuthority'),
]);

export const MINT_LEN = MintLayout.span;
