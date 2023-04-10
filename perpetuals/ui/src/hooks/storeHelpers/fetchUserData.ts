import { PoolAccount } from "@/lib/PoolAccount";
import { TokenE } from "@/lib/Token";
import { UserAccount } from "@/lib/UserAccount";
import { fetchLPBalance, fetchTokenBalance } from "@/utils/retrieveData";
import { Connection, PublicKey } from "@solana/web3.js";

export default async function getUserLpAll(
  connection: Connection,
  publicKey: PublicKey,
  poolData: Record<string, PoolAccount>
): Promise<Record<string, number>> {
  let lpTokenAccounts: Record<string, number> = {};
  let promises = Object.values(poolData).map(async (pool) => {
    lpTokenAccounts[pool.address.toString()] = await fetchLPBalance(
      pool.getLpTokenMint(),
      publicKey,
      connection
    );
  });

  await Promise.all(promises);

  return lpTokenAccounts;
}

export async function getUserLpSingle() {}

export async function getUserTokenAll(
  connection: Connection,
  publicKey: PublicKey,
  poolData: Record<string, PoolAccount>
): Promise<Record<TokenE, number>> {
  let tokens: TokenE[] = [];

  Object.values(poolData).map(async (pool) => {
    tokens.push(...pool.getTokenList());
  });

  tokens = Array.from(new Set(tokens));

  let tokenBalances: Record<string, number> = {};

  let promises = tokens.map(async (token) => {
    tokenBalances[token] = await fetchTokenBalance(
      token,
      publicKey,
      connection
    );
  });
  await Promise.all(promises);

  return tokenBalances;
}

export async function getUserTokenSingle() {}

export async function getAllUserData(
  connection: Connection,
  publicKey: PublicKey,
  poolData: Record<string, PoolAccount>
): Promise<UserAccount> {
  let lpBalances = await getUserLpAll(connection, publicKey, poolData);

  let tokenBalances = await getUserTokenAll(connection, publicKey, poolData);

  let userData = new UserAccount(lpBalances, tokenBalances);

  return userData;
}
