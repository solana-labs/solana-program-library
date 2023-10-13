import { getAddressCodec } from '@solana/web3.js';

import { PoolAddress, VoteAccountAddress } from './addresses';

export * from './addresses';
export * from './instructions';
export * from './transactions';

export async function getVoteAccountAddressForPool(
  rpc: any, // XXX not exported: Rpc<GetAccountInfoApi>,
  poolAddress: PoolAddress,
  abortSignal?: AbortSignal,
): Promise<VoteAccountAddress> {
  const poolAccount = await rpc.getAccountInfo(poolAddress).send(abortSignal);
  if (!(poolAccount && poolAccount.data[0] === 1)) {
    throw 'invalid pool address';
  }
  return getAddressCodec().deserialize(poolAccount.data.slice(1))[0] as VoteAccountAddress;
}
