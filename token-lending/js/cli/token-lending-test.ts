/* eslint-disable @typescript-eslint/no-unsafe-assignment */
/* eslint-disable @typescript-eslint/no-unsafe-member-access */

import { Account, Connection } from "@solana/web3.js";
import { Token, TOKEN_PROGRAM_ID } from "@solana/spl-token";

import { LENDING_PROGRAM_ID, LendingMarket } from "../client";
import { newAccountWithLamports } from "../client/util/new-account-with-lamports";
import { url } from "../client/util/url";

let connection: Connection | undefined;
async function getConnection(): Promise<Connection> {
  if (connection) return connection;

  connection = new Connection(url, "recent");
  const version = await connection.getVersion();

  console.log("Connection to cluster established:", url, version);
  return connection;
}

export async function createLendingMarket(): Promise<void> {
  const connection = await getConnection();

  const payer = await newAccountWithLamports(
    connection,
    100000000000 /* wag */
  );

  console.log("creating quote token mint");
  const quoteMintAuthority = new Account();
  const quoteTokenMint = await Token.createMint(
    connection,
    payer,
    quoteMintAuthority.publicKey,
    null,
    2,
    TOKEN_PROGRAM_ID
  );

  const lendingMarketAccount = new Account();
  await LendingMarket.create({
    connection,
    tokenProgramId: TOKEN_PROGRAM_ID,
    lendingProgramId: LENDING_PROGRAM_ID,
    quoteTokenMint: quoteTokenMint.publicKey,
    lendingMarketAccount,
    lendingMarketOwner: payer.publicKey,
    payer,
  });
}
