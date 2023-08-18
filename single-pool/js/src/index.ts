import { Connection, PublicKey } from '@solana/web3.js';

export * from './mpl_metadata';
export * from './addresses';
export * from './instructions';
export * from './transactions';

export async function getVoteAccountAddressForPool(connection: Connection, poolAccount: PublicKey) {
  const poolData = (await connection.getAccountInfo(poolAccount)).data;
  return new PublicKey(poolData.slice(1));
}
