// @flow

import fs from 'mz/fs';
import semver from 'semver';
import { Account, Connection, BpfLoader, PublicKey, BPF_LOADER_PROGRAM_ID } from '@solana/web3.js';

import { Token } from '../../../token/js/client/token';
import { TokenSwap } from '../client/token-swap';
import { Store } from '../client/util/store';
import { newAccountWithLamports } from '../client/util/new-account-with-lamports';
import { url } from '../url';
import { sleep } from '../client/util/sleep';

// The following globals are created by `createTokenSwap` and used by subsequent tests
// Token swap
let tokenSwap: TokenSwap;
// authority of the token and accounts
let authority: PublicKey;
// nonce used to generate the authority public key
let nonce: number;
// owner of the user accounts
let owner: Account;
// Token pool
let tokenPool: Token;
let tokenAccountPool: PublicKey;
// Tokens swapped
let mintA: Token;
let mintB: Token;
let tokenAccountA: PublicKey;
let tokenAccountB: PublicKey;

// Initial amount in each swap token
const BASE_AMOUNT = 1000;
// Amount passed to instructions
const USER_AMOUNT = 100;

function assert(condition, message) {
  if (!condition) {
    console.log(Error().stack + ':token-test.js');
    throw message || 'Assertion failed';
  }
}

let connection;
async function getConnection(): Promise<Connection> {
  if (connection) return connection;

  let newConnection = new Connection(url, 'recent',);
  const version = await newConnection.getVersion();

  // commitment params are only supported >= 0.21.0
  const solanaCoreVersion = version['solana-core'].split(' ')[0];
  if (semver.gte(solanaCoreVersion, '0.21.0')) {
    newConnection = new Connection(url, 'recent');
  }

  // eslint-disable-next-line require-atomic-updates
  connection = newConnection;
  console.log('Connection to cluster established:', url, version);
  return newConnection;
}

async function loadProgram(connection: Connection, path: string): Promise<PublicKey> {
  const NUM_RETRIES = 500; /* allow some number of retries */
  const data = await fs.readFile(path
  );
  const { feeCalculator } = await connection.getRecentBlockhash();
  const balanceNeeded =
    feeCalculator.lamportsPerSignature *
    (BpfLoader.getMinNumSignatures(data.length) + NUM_RETRIES) +
    (await connection.getMinimumBalanceForRentExemption(data.length));

  const from = await newAccountWithLamports(connection, balanceNeeded);
  const program_account = new Account();
  console.log('Loading program:', path);
  await BpfLoader.load(connection, from, program_account, data, BPF_LOADER_PROGRAM_ID);
  return program_account.publicKey;
}

async function GetPrograms(connection: Connection): Promise<[PublicKey, PublicKey]> {
  const store = new Store();
  let tokenProgramId = null;
  let tokenSwapProgramId = null;
  try {
    const config = await store.load('config.json');
    console.log('Using pre-loaded Token and Token-swap programs');
    console.log('  Note: To reload programs remove client/util/store/config.json');
    tokenProgramId = new PublicKey(config.tokenProgramId);
    tokenSwapProgramId = new PublicKey(config.tokenSwapProgramId);
  } catch (err) {
    tokenProgramId = await loadProgram(connection, '../../target/bpfel-unknown-unknown/release/spl_token.so');
    tokenSwapProgramId = await loadProgram(connection, '../../target/bpfel-unknown-unknown/release/spl_token_swap.so');
    await store.save('config.json', {
      tokenProgramId: tokenProgramId.toString(),
      tokenSwapProgramId: tokenSwapProgramId.toString()
    });
  }
  return [tokenProgramId, tokenSwapProgramId];
}

export async function loadPrograms(): Promise<void> {
  const connection = await getConnection();
  const [tokenProgramId, tokenSwapProgramId] = await GetPrograms(connection);

  console.log('Token Program ID', tokenProgramId.toString());
  console.log('Token-swap Program ID', tokenSwapProgramId.toString());
}

export async function createTokenSwap(): Promise<void> {
  const connection = await getConnection();
  const [tokenProgramId, tokenSwapProgramId] = await GetPrograms(connection);
  const payer = await newAccountWithLamports(connection, 100000000000 /* wag */);
  owner = await newAccountWithLamports(connection, 100000000000 /* wag */);
  const tokenSwapAccount = new Account();

  [authority, nonce] = await PublicKey.findProgramAddress(
    [tokenSwapAccount.publicKey.toBuffer()],
    tokenSwapProgramId
  );

  console.log('creating pool mint');
  tokenPool = await Token.createMint(
    connection,
    payer,
    authority,
    null,
    2,
    tokenProgramId,
  );

  console.log('creating pool account');
  tokenAccountPool = await tokenPool.createAccount(owner.publicKey);

  console.log('creating token A');
  mintA = await Token.createMint(
    connection,
    payer,
    owner.publicKey,
    null,
    2,
    tokenProgramId,
  );

  console.log('creating token A account');
  tokenAccountA = await mintA.createAccount(authority);
  console.log('minting token A to swap');
  await mintA.mintTo(tokenAccountA, owner, [], BASE_AMOUNT);

  console.log('creating token B');
  mintB = await Token.createMint(
    connection,
    payer,
    owner.publicKey,
    null,
    2,
    tokenProgramId,
  );

  console.log('creating token B account');
  tokenAccountB = await mintB.createAccount(authority);
  console.log('minting token B to swap');
  await mintB.mintTo(tokenAccountB, owner, [], BASE_AMOUNT);

  console.log('creating token swap');
  const swapPayer = await newAccountWithLamports(connection, 100000000000 /* wag */);
  tokenSwap = await TokenSwap.createTokenSwap(
    connection,
    swapPayer,
    tokenSwapAccount,
    authority,
    tokenAccountA,
    tokenAccountB,
    tokenPool.publicKey,
    tokenAccountPool,
    tokenProgramId,
    nonce,
    1,
    4,
    tokenSwapProgramId
  );

  console.log('getting token swap');
  const swapInfo = await tokenSwap.getInfo();
  assert(swapInfo.tokenAccountA.equals(tokenAccountA));
  assert(swapInfo.tokenAccountB.equals(tokenAccountB));
  assert(swapInfo.tokenPool.equals(tokenPool.publicKey));
  assert(1 == swapInfo.feesNumerator.toNumber());
  assert(4 == swapInfo.feesDenominator.toNumber());
}

export async function deposit(): Promise<void> {
  let userAccountA = await mintA.createAccount(owner.publicKey);
  await mintA.mintTo(userAccountA, owner, [], USER_AMOUNT);
  await mintA.approve(
    userAccountA,
    authority,
    owner,
    [],
    USER_AMOUNT,
  );
  let userAccountB = await mintB.createAccount(owner.publicKey);
  await mintB.mintTo(userAccountB, owner, [], USER_AMOUNT);
  await mintB.approve(
    userAccountB,
    authority,
    owner,
    [],
    USER_AMOUNT,
  );
  let newAccountPool = await tokenPool.createAccount(owner.publicKey);
  const [tokenProgramId,] = await GetPrograms(connection);

  await tokenSwap.deposit(
    authority,
    userAccountA,
    userAccountB,
    tokenAccountA,
    tokenAccountB,
    tokenPool.publicKey,
    newAccountPool,
    tokenProgramId,
    USER_AMOUNT,
  );

  let info;
  info = await mintA.getAccountInfo(userAccountA);
  console.log('userAccountA', info.amount.toNumber());
  assert(info.amount.toNumber() == 0);
  assert(info.amount.toNumber() == 0);
  info = await mintB.getAccountInfo(userAccountB);
  console.log('userAccountB', info.amount.toNumber());
  assert(info.amount.toNumber() == 0);
  info = await mintA.getAccountInfo(tokenAccountA);
  console.log('tokenAccountA', info.amount.toNumber());
  assert(info.amount.toNumber() == BASE_AMOUNT + USER_AMOUNT);
  info = await mintB.getAccountInfo(tokenAccountB);
  console.log('tokenAccountB', info.amount.toNumber());
  assert(info.amount.toNumber() == BASE_AMOUNT + USER_AMOUNT);
  info = await tokenPool.getAccountInfo(newAccountPool);
  console.log('newAccountPool', info.amount.toNumber());
  assert(info.amount.toNumber() == USER_AMOUNT);
}

export async function withdraw(): Promise<void> {
  let userAccountA = await mintA.createAccount(owner.publicKey);
  let userAccountB = await mintB.createAccount(owner.publicKey);
  await tokenPool.approve(
    tokenAccountPool,
    authority,
    owner,
    [],
    USER_AMOUNT,
  );
  const [tokenProgramId,] = await GetPrograms(connection);

  await tokenSwap.withdraw(
    authority,
    tokenAccountPool,
    tokenAccountA,
    tokenAccountB,
    userAccountA,
    userAccountB,
    tokenProgramId,
    USER_AMOUNT
  );

  let info;
  info = await tokenPool.getAccountInfo(tokenAccountPool);
  console.log('tokenAccountPool', info.amount.toNumber());
  assert(info.amount.toNumber() == BASE_AMOUNT - USER_AMOUNT);
  info = await mintA.getAccountInfo(tokenAccountA);
  console.log('tokenAccountA', info.amount.toNumber());
  assert(info.amount.toNumber() == BASE_AMOUNT);
  info = await mintB.getAccountInfo(tokenAccountB);
  console.log('tokenAccountB', info.amount.toNumber());
  assert(info.amount.toNumber() == BASE_AMOUNT);
  info = await mintA.getAccountInfo(userAccountA);
  console.log('userAccountA', info.amount.toNumber());
  assert(info.amount.toNumber() == USER_AMOUNT);
  info = await mintB.getAccountInfo(userAccountB);
  console.log('userAccountB', info.amount.toNumber());
  assert(info.amount.toNumber() == USER_AMOUNT);
}

export async function swap(): Promise<void> {
  let userAccountA = await mintA.createAccount(owner.publicKey);
  await mintA.mintTo(userAccountA, owner, [], USER_AMOUNT);
  await mintA.approve(
    userAccountA,
    authority,
    owner,
    [],
    USER_AMOUNT,
  );
  let userAccountB = await mintB.createAccount(owner.publicKey);
  const [tokenProgramId,] = await GetPrograms(connection);

  await tokenSwap.swap(
    authority,
    userAccountA,
    tokenAccountA,
    tokenAccountB,
    userAccountB,
    tokenProgramId,
    USER_AMOUNT,
  );
  await sleep(500);
  let info;
  info = await mintA.getAccountInfo(userAccountA);
  console.log('userAccountA', info.amount.toNumber());
  assert(info.amount.toNumber() == 0);
  info = await mintA.getAccountInfo(tokenAccountA);
  console.log('tokenAccountA', info.amount.toNumber());
  assert(info.amount.toNumber() == BASE_AMOUNT + USER_AMOUNT);
  info = await mintB.getAccountInfo(tokenAccountB);
  console.log('tokenAccountB', info.amount.toNumber());
  assert(info.amount.toNumber() == 931);
  info = await mintB.getAccountInfo(userAccountB);
  console.log('userAccountB', info.amount.toNumber());
  assert(info.amount.toNumber() == 69);
  info = await tokenPool.getAccountInfo(tokenAccountPool);
  console.log('tokenAccountPool', info.amount.toNumber());
  assert(info.amount.toNumber() == BASE_AMOUNT - USER_AMOUNT);
}
