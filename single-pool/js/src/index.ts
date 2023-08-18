import { Connection, PublicKey } from '@solana/web3.js';

export * from './mpl_metadata';
export * from './addresses';
export * from './instructions';
export * from './transactions';

export async function getVoteAccountAddressForPool(connection: Connection, poolAddress: PublicKey) {
  const poolAccount = await connection.getAccountInfo(poolAddress);
  if (!poolAccount) {
    throw 'invalid pool address';
  }

  return new PublicKey(poolAccount.data.slice(1));
}
