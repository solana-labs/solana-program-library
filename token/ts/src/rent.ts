import { Commitment, Connection } from '@solana/web3.js';
import { ACCOUNT_LEN } from './account';
import { MINT_LEN } from './mint';
import { MULTISIG_LEN } from './multisig';

/**
 * Get the minimum balance for a mint to be rent exempt
 *
 * @return Number of lamports required
 */
export async function getMinimumBalanceForRentExemptMint(
    connection: Connection,
    commitment?: Commitment
): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(MINT_LEN, commitment);
}

/**
 * Get the minimum balance for the account to be rent exempt
 *
 * @return Number of lamports required
 */
export async function getMinimumBalanceForRentExemptAccount(
    connection: Connection,
    commitment?: Commitment
): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(ACCOUNT_LEN, commitment);
}

/**
 * Get the minimum balance for the multsig to be rent exempt
 *
 * @return Number of lamports required
 */
export async function getMinimumBalanceForRentExemptMultisig(
    connection: Connection,
    commitment?: Commitment
): Promise<number> {
    return await connection.getMinimumBalanceForRentExemption(MULTISIG_LEN, commitment);
}
