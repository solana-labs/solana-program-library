// @flow

import {Keypair, Connection, Signer} from '@solana/web3.js';

/**
 * Create a new system account and airdrop it some lamports
 *
 * @private
 */
export async function newSystemAccountWithAirdrop(
  connection: Connection,
  lamports: number = 1,
): Promise<Signer> {
  const account = Keypair.generate();
  await connection.requestAirdrop(account.publicKey, lamports);
  return account;
}
