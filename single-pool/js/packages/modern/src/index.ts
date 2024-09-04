import { getAddressCodec } from '@solana/addresses';

import { PoolAddress, VoteAccountAddress } from './addresses.js';

export * from './addresses.js';
export * from './instructions.js';
export * from './transactions.js';

export async function getVoteAccountAddressForPool(
  rpc: any, // XXX not exported: Rpc<GetAccountInfoApi>,
  poolAddress: PoolAddress,
  abortSignal?: AbortSignal,
): Promise<VoteAccountAddress> {
  const poolAccount = await rpc.getAccountInfo(poolAddress).send(abortSignal);
  if (!(poolAccount && poolAccount.data[0] === 1)) {
    throw 'invalid pool address';
  }
  return getAddressCodec().decode(poolAccount.data.slice(1)) as VoteAccountAddress;
}
