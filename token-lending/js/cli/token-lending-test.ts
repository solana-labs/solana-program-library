/* eslint-disable @typescript-eslint/no-unsafe-assignment */
/* eslint-disable @typescript-eslint/no-unsafe-member-access */

import fs from "mz/fs";
import {
  Account,
  Connection,
  BpfLoader,
  PublicKey,
  BPF_LOADER_PROGRAM_ID,
} from "@solana/web3.js";
import { Token } from "@solana/spl-token";

import { LendingMarket } from "../client";
import { Store } from "../client/util/store";
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

let tokenProgramId: PublicKey;
let tokenLendingProgramId: PublicKey;

export async function loadPrograms(): Promise<void> {
  const connection = await getConnection();
  ({ tokenProgramId, tokenLendingProgramId } = await GetPrograms(connection));

  console.log("SPL Token Program ID", tokenProgramId.toString());
  console.log("SPL Token Lending Program ID", tokenLendingProgramId.toString());
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
    tokenProgramId
  );

  const lendingMarketAccount = new Account();
  await LendingMarket.create({
    connection,
    tokenProgramId,
    lendingProgramId: tokenLendingProgramId,
    quoteTokenMint: quoteTokenMint.publicKey,
    lendingMarketAccount,
    lendingMarketOwner: payer.publicKey,
    payer,
  });
}

async function loadProgram(
  connection: Connection,
  path: string
): Promise<PublicKey> {
  const data = await fs.readFile(path);
  const { feeCalculator } = await connection.getRecentBlockhash();

  const loaderCost =
    feeCalculator.lamportsPerSignature *
    BpfLoader.getMinNumSignatures(data.length);
  const minAccountBalance = await connection.getMinimumBalanceForRentExemption(
    0
  );
  const minExecutableBalance = await connection.getMinimumBalanceForRentExemption(
    data.length
  );
  const balanceNeeded = minAccountBalance + loaderCost + minExecutableBalance;

  const from = await newAccountWithLamports(connection, balanceNeeded);
  const program_account = new Account();
  console.log("Loading program:", path);
  await BpfLoader.load(
    connection,
    from,
    program_account,
    data,
    BPF_LOADER_PROGRAM_ID
  );
  return program_account.publicKey;
}

async function GetPrograms(
  connection: Connection
): Promise<{
  tokenProgramId: PublicKey;
  tokenLendingProgramId: PublicKey;
}> {
  const store = new Store();
  let tokenProgramId = null;
  let tokenLendingProgramId = null;
  try {
    const config = await store.load("config.json");
    console.log("Using pre-loaded programs");
    console.log(
      "  Note: To reload programs remove client/util/store/config.json"
    );
    if ("tokenProgramId" in config && "tokenLendingProgramId" in config) {
      tokenProgramId = new PublicKey(config["tokenProgramId"]);
      tokenLendingProgramId = new PublicKey(config["tokenLendingProgramId"]);
    } else {
      throw new Error("Program ids not found");
    }
  } catch (err) {
    tokenProgramId = await loadProgram(
      connection,
      "../../target/bpfel-unknown-unknown/release/spl_token.so"
    );
    tokenLendingProgramId = await loadProgram(
      connection,
      "../../target/bpfel-unknown-unknown/release/spl_token_lending.so"
    );
    await store.save("config.json", {
      tokenProgramId: tokenProgramId.toString(),
      tokenLendingProgramId: tokenLendingProgramId.toString(),
    });
  }
  return { tokenProgramId, tokenLendingProgramId };
}
