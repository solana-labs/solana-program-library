import { struct, u32, u8 } from '@solana/buffer-layout';
import { bool, publicKey, u64 } from '@solana/buffer-layout-utils';
import { Commitment, Connection, PublicKey } from '@solana/web3.js';
import { ASSOCIATED_TOKEN_PROGRAM_ID, TOKEN_PROGRAM_ID, TokenError } from '../constants';

/** Information about a mint */
export interface Mint {
    /** The address of this mint */
    address: PublicKey;
    /**
     * Optional authority used to mint new tokens. The mint authority may only be provided during mint creation.
     * If no mint authority is present then the mint has a fixed supply and no further tokens may be minted.
     */
    mintAuthority: PublicKey | null;
    /** Total supply of tokens */
    supply: bigint;
    /** Number of base 10 digits to the right of the decimal place */
    decimals: number;
    /** Is this mint initialized */
    isInitialized: boolean;
    /** Optional authority to freeze token accounts */
    freezeAuthority: PublicKey | null;
}

/** @TODO: document */
export interface RawMint {
    mintAuthority: PublicKey | null;
    supply: bigint;
    decimals: number;
    isInitialized: boolean;
    freezeAuthority: PublicKey | null;
}

/** @TODO: document */
export const MintLayout = struct<RawMint>([
    u32('mintAuthorityOption'),
    publicKey('mintAuthority'),
    u64('supply'),
    u8('decimals'),
    bool('isInitialized'),
    u32('freezeAuthorityOption'),
    publicKey('freezeAuthority'),
]);

/** @TODO: document */
export const MINT_LEN = MintLayout.span;

/** Get the minimum lamport balance for a Mint to be rent exempt
 *
 * @param connection @TODO: docs
 * @param commitment @TODO: docs
 *
 * @return amount of lamports required
 */
export async function getMinimumBalanceForRentExemptMint(
    connection: Connection,
    commitment?: Commitment
): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(MINT_LEN, commitment);
}

/**
 * Retrieve mint information
 *
 * @TODO: docs
 */
export async function getMintInfo(
    connection: Connection,
    address: PublicKey,
    programId = TOKEN_PROGRAM_ID
): Promise<Mint> {
    const info = await connection.getAccountInfo(address);
    if (!info) throw new Error(TokenError.ACCOUNT_NOT_FOUND);
    if (!info.owner.equals(programId)) throw new Error(TokenError.INVALID_ACCOUNT_OWNER);
    if (info.data.length != MINT_LEN) throw new Error(TokenError.INVALID_ACCOUNT_SIZE);

    const mintInfo = MintLayout.decode(Buffer.from(info.data));

    // @FIXME
    if (!mintInfo.mintAuthorityOption) {
        mintInfo.mintAuthority = null;
    }

    if (!mintInfo.freezeAuthorityOption) {
        mintInfo.freezeAuthority = null;
    }

    return mintInfo;
}

/**
 * Get the address of the associated token account for a given mint and owner
 *
 * @param mint                     Token mint account
 * @param owner                    Owner of the new account
 * @param allowOwnerOffCurve       @TODO: docs
 * @param programId                SPL Token program account
 * @param associatedTokenProgramId SPL Associated Token program account
 *
 * @return Address of the associated token account
 */
export async function getAssociatedTokenAddress(
    mint: PublicKey,
    owner: PublicKey,
    allowOwnerOffCurve = false,
    programId = TOKEN_PROGRAM_ID,
    associatedTokenProgramId = ASSOCIATED_TOKEN_PROGRAM_ID
): Promise<PublicKey> {
    if (!allowOwnerOffCurve && !PublicKey.isOnCurve(owner.toBuffer())) {
        throw new Error(`Owner cannot sign: ${owner.toString()}`);
    }

    const [address] = await PublicKey.findProgramAddress(
        [owner.toBuffer(), programId.toBuffer(), mint.toBuffer()],
        associatedTokenProgramId
    );

    return address;
}
