// @flow

import fs from 'mz/fs';
import semver from 'semver';
import { Account, Connection, BpfLoader, PublicKey } from '@solana/web3.js';

import { Token, TokenAmount } from '../../../token/js/client/token';
import { TokenSwap } from '../client/token-swap';
import { Store } from '../client/util/store';
import { newAccountWithLamports } from '../client/util/new-account-with-lamports';
import { url } from '../url';
import { sleep } from '../client/util/sleep';

// The following globals are created by `createNewTokenSwap` and used by subsequent tests
// Token swap
let tokenSwap: TokenSwap;
// authority of the token and accounts
let authority: PublicKey;
// owner of the user accounts
let owner: Account;
// Token pool
let tokenPool: Token;
let tokenAccountPool: PublicKey;
// Tokens swapped
let tokenA: Token;
let tokenB: Token;
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
  await BpfLoader.load(connection, from, program_account, data);
  return program_account.publicKey;
}

async function GetPrograms(connection: Connection): Promise<[PublicKey, PublicKey]> {
  const store = new Store();
  let tokenProgramId = null;
  let tokenSwapProgramId = null;
  try {
    const config = await store.load('config.json');
    console.log('Using pre-loaded Token and Token-swap programs');
    console.log('  Note: To reload programs remove client/util/sore/config.json');
    tokenProgramId = new PublicKey(config.tokenProgramId);
    tokenSwapProgramId = new PublicKey(config.tokenSwapProgramId);
  } catch (err) {
    tokenProgramId = await loadProgram(connection, '../../token/target/bpfel-unknown-unknown/release/spl_token.so');
    tokenSwapProgramId = await loadProgram(connection, '../target/bpfel-unknown-unknown/release/spl_token_swap.so');
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
  const payer = await Token.getAccount(connection);
  owner = await Token.getAccount(connection);
  const tokenSwapAccount = new Account();
  authority = await PublicKey.createProgramAddress(
    [tokenSwapAccount.publicKey.toString().substring(0, 32)],
    tokenSwapProgramId
  );

  // create pool
  [tokenPool, tokenAccountPool] = await Token.createNewToken(
    connection,
    payer,
    authority,
    owner.publicKey,
    new TokenAmount(0),
    2,
    tokenProgramId,
    true,
  );

  // create token A
  [tokenA, tokenAccountA] = await Token.createNewToken(
    connection,
    payer,
    owner.publicKey,
    authority,
    new TokenAmount(BASE_AMOUNT),
    2,
    tokenProgramId,
    true,
  );

  // create token B
  [tokenB, tokenAccountB] = await Token.createNewToken(
    connection,
    payer,
    owner.publicKey,
    authority,
    new TokenAmount(BASE_AMOUNT),
    2,
    tokenProgramId,
    true,
  );

  // create token swap
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
    1,
    4,
    tokenSwapProgramId
  );

  const swapInfo = await tokenSwap.getInfo();
  assert(swapInfo.tokenAccountA.equals(tokenAccountA));
  assert(swapInfo.tokenAccountB.equals(tokenAccountB));
  assert(swapInfo.tokenPool.equals(tokenPool.publicKey));
  assert(1 == swapInfo.feesNumerator.toNumber());
  assert(4 == swapInfo.feesDenominator.toNumber());
}

export async function deposit(): Promise<void> {
  let userAccountA = await tokenA.newAccount(owner.publicKey);
  await tokenA.mintTo(owner, userAccountA, USER_AMOUNT);
  let delegateAccountA = await tokenA.newAccount(authority, userAccountA);
  await tokenA.approve(
    owner,
    userAccountA,
    delegateAccountA,
    USER_AMOUNT,
  );
  let userAccountB = await tokenB.newAccount(owner.publicKey);
  await tokenB.mintTo(owner, userAccountB, USER_AMOUNT);
  let delegateAccountB = await tokenB.newAccount(authority, userAccountB);
  await tokenB.approve(
    owner,
    userAccountB,
    delegateAccountB,
    USER_AMOUNT,
  );
  let newAccountPool = await tokenPool.newAccount(owner.publicKey);
  const [tokenProgramId,] = await GetPrograms(connection);

  await tokenSwap.deposit(
    authority,
    delegateAccountA,
    userAccountA,
    delegateAccountB,
    userAccountB,
    tokenAccountA,
    tokenAccountB,
    tokenPool.publicKey,
    newAccountPool,
    tokenProgramId,
    USER_AMOUNT,
  );

  let info;
  info = await tokenA.getAccountInfo(delegateAccountA);
  console.log('delegageAccountA', info.amount.toNumber());
  assert(info.amount.toNumber() == 0);
  info = await tokenA.getAccountInfo(userAccountA);
  console.log('userAccountA', info.amount.toNumber());
  assert(info.amount.toNumber() == 0);
  info = await tokenB.getAccountInfo(delegateAccountB);
  console.log('delegageAccountB', info.amount.toNumber());
  assert(info.amount.toNumber() == 0);
  info = await tokenB.getAccountInfo(userAccountB);
  console.log('userAccountB', info.amount.toNumber());
  assert(info.amount.toNumber() == 0);
  info = await tokenA.getAccountInfo(tokenAccountA);
  console.log('tokenAccountA', info.amount.toNumber());
  assert(info.amount.toNumber() == BASE_AMOUNT + USER_AMOUNT);
  info = await tokenB.getAccountInfo(tokenAccountB);
  console.log('tokenAccountB', info.amount.toNumber());
  assert(info.amount.toNumber() == BASE_AMOUNT + USER_AMOUNT);
  info = await tokenPool.getAccountInfo(newAccountPool);
  console.log('newAccountPool', info.amount.toNumber());
  assert(info.amount.toNumber() == USER_AMOUNT);
}

export async function withdraw(): Promise<void> {
  let userAccountA = await tokenA.newAccount(owner.publicKey);
  let userAccountB = await tokenB.newAccount(owner.publicKey);
  let delegateAccountPool = await tokenPool.newAccount(authority, tokenAccountPool);
  await tokenPool.approve(
    owner,
    tokenAccountPool,
    delegateAccountPool,
    USER_AMOUNT,
  );
  const [tokenProgramId,] = await GetPrograms(connection);

  await tokenSwap.withdraw(
    authority,
    delegateAccountPool,
    tokenAccountPool,
    tokenPool.publicKey,
    tokenAccountA,
    tokenAccountB,
    userAccountA,
    userAccountB,
    tokenProgramId,
    USER_AMOUNT
  );

  let info;
  info = await tokenPool.getAccountInfo(delegateAccountPool);
  console.log('delegateAccountPool', info.amount.toNumber());
  assert(info.amount.toNumber() == 0);
  info = await tokenPool.getAccountInfo(tokenAccountPool);
  console.log('tokenAccountPool', info.amount.toNumber());
  assert(info.amount.toNumber() == BASE_AMOUNT - USER_AMOUNT);
  info = await tokenA.getAccountInfo(tokenAccountA);
  console.log('tokenAccountA', info.amount.toNumber());
  assert(info.amount.toNumber() == BASE_AMOUNT);
  info = await tokenB.getAccountInfo(tokenAccountB);
  console.log('tokenAccountB', info.amount.toNumber());
  assert(info.amount.toNumber() == BASE_AMOUNT);
  info = await tokenA.getAccountInfo(userAccountA);
  console.log('userAccountA', info.amount.toNumber());
  assert(info.amount.toNumber() == USER_AMOUNT);
  info = await tokenB.getAccountInfo(userAccountB);
  console.log('userAccountB', info.amount.toNumber());
  assert(info.amount.toNumber() == USER_AMOUNT);
}

export async function swap(): Promise<void> {
  let userAccountA = await tokenA.newAccount(owner.publicKey);
  await tokenA.mintTo(owner, userAccountA, USER_AMOUNT);
  let delegateAccountA = await tokenA.newAccount(authority, userAccountA);
  await tokenA.approve(
    owner,
    userAccountA,
    delegateAccountA,
    USER_AMOUNT,
  );
  let userAccountB = await tokenB.newAccount(owner.publicKey);
  const [tokenProgramId,] = await GetPrograms(connection);

  await tokenSwap.swap(
    authority,
    delegateAccountA,
    userAccountA,
    tokenAccountA,
    tokenAccountB,
    userAccountB,
    tokenProgramId,
    USER_AMOUNT,
  );
  await sleep(500);
  let info;
  info = await tokenA.getAccountInfo(userAccountA);
  console.log('userAccountA', info.amount.toNumber());
  assert(info.amount.toNumber() == 0);
  info = await tokenA.getAccountInfo(tokenAccountA);
  console.log('tokenAccountA', info.amount.toNumber());
  assert(info.amount.toNumber() == BASE_AMOUNT + USER_AMOUNT);
  info = await tokenB.getAccountInfo(tokenAccountB);
  console.log('tokenAccountB', info.amount.toNumber());
  assert(info.amount.toNumber() == 931);
  info = await tokenB.getAccountInfo(userAccountB);
  console.log('userAccountB', info.amount.toNumber());
  assert(info.amount.toNumber() == 69);
  info = await tokenPool.getAccountInfo(tokenAccountPool);
  console.log('tokenAccountPool', info.amount.toNumber());
  assert(info.amount.toNumber() == BASE_AMOUNT - USER_AMOUNT);
}
