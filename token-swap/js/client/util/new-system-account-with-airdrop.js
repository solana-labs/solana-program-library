// @flow
import { Account } from '@solana/web3.js';
/**
 * Create a new system account and airdrop it some lamports
 *
 * @private
 */
export async function newSystemAccountWithAirdrop(connection, lamports = 1) {
    const account = new Account();
    await connection.requestAirdrop(account.publicKey, lamports);
    return account;
}
