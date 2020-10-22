import { Account, Connection } from "@solana/web3.js";

export async function newAccountWithLamports(
  connection: Connection,
  lamports = 1000000
): Promise<Account> {
  const account = new Account();
  const signature = await connection.requestAirdrop(
    account.publicKey,
    lamports
  );
  await connection.confirmTransaction(signature, "singleGossip");
  return account;
}
