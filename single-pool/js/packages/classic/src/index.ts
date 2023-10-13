import { Connection, PublicKey } from '@solana/web3.js';
import { getVoteAccountAddressForPool as getVoteModern } from '@solana/single-pool';

import { rpc } from './internal';

export * from './mpl_metadata';
export * from './addresses';
export * from './instructions';
export * from './transactions';

export async function getVoteAccountAddressForPool(connection: Connection, poolAddress: PublicKey) {
  const voteAccountModern = await getVoteModern(rpc(connection), poolAddress.toBase58());

  return new PublicKey(voteAccountModern);
}
