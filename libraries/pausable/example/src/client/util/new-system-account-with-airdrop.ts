import {Account, Connection} from '@solana/web3.js';

/**
 * Create a new system account and airdrop it some lamports
 *
 * @private
 */
export async function newSystemAccountWithAirdrop(
  connection: Connection,
  lamports = 1,
): Promise<Account> {
  const account = new Account();
  await connection.requestAirdrop(account.publicKey, lamports);
  return account;
}
