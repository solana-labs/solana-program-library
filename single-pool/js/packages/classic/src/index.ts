import { Connection, PublicKey } from '@solana/web3.js';
import { getVoteAccountAddressForPool as getVoteModern } from '@solana/spl-single-pool';
import type { PoolAddress } from '@solana/spl-single-pool';

import { rpc } from './internal.js';

export * from './mpl_metadata.js';
export * from './addresses.js';
export * from './instructions.js';
export * from './transactions.js';

export async function getVoteAccountAddressForPool(connection: Connection, poolAddress: PublicKey) {
  const voteAccountModern = await getVoteModern(
    rpc(connection),
    poolAddress.toBase58() as PoolAddress,
  );

  return new PublicKey(voteAccountModern);
}
