// @flow

import fs from 'mz/fs';
import {
  Account,
  Connection,
  BpfLoader,
  PublicKey,
  BPF_LOADER_PROGRAM_ID,
} from '@solana/web3.js';

import {Token} from '../../../token/js/client/token';
import {TokenSwap} from '../client/token-swap';
import {Store} from '../client/util/store';
import {newAccountWithLamports} from '../client/util/new-account-with-lamports';
import {url} from '../url';
import {sleep} from '../client/util/sleep';

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
// Amount passed to swap instruction
const SWAP_AMOUNT_IN = 100;
const SWAP_AMOUNT_OUT = 70;
// Pool token amount minted on init
const DEFAULT_POOL_TOKEN_AMOUNT = 1000000000;
// Pool token amount to withdraw / deposit
const POOL_TOKEN_AMOUNT = 1000000;

function assert(condition, message) {
  if (!condition) {
    console.log(Error().stack + ':token-test.js');
    throw message || 'Assertion failed';
  }
}

let connection;
async function getConnection(): Promise<Connection> {
  if (connection) return connection;

  connection = new Connection(url, 'recent');
  const version = await connection.getVersion();

  console.log('Connection to cluster established:', url, version);
  return connection;
}

async function loadProgram(
  connection: Connection,
  path: string,
): Promise<PublicKey> {
  const NUM_RETRIES = 500; /* allow some number of retries */
  const data = await fs.readFile(path);
  const {feeCalculator} = await connection.getRecentBlockhash();
  const balanceNeeded =
    feeCalculator.lamportsPerSignature *
      (BpfLoader.getMinNumSignatures(data.length) + NUM_RETRIES) +
    (await connection.getMinimumBalanceForRentExemption(data.length));

  const from = await newAccountWithLamports(connection, balanceNeeded);
  const program_account = new Account();
  console.log('Loading program:', path);
  await BpfLoader.load(
    connection,
    from,
    program_account,
    data,
    BPF_LOADER_PROGRAM_ID,
  );
  return program_account.publicKey;
}

async function GetPrograms(
  connection: Connection,
): Promise<[PublicKey, PublicKey]> {
  const store = new Store();
  let tokenProgramId = null;
  let tokenSwapProgramId = null;
  try {
    const config = await store.load('config.json');
    console.log('Using pre-loaded Token and Token-swap programs');
    console.log(
      '  Note: To reload programs remove client/util/store/config.json',
    );
    tokenProgramId = new PublicKey(config.tokenProgramId);
    tokenSwapProgramId = new PublicKey(config.tokenSwapProgramId);
  } catch (err) {
    tokenProgramId = await loadProgram(
      connection,
      '../../target/bpfel-unknown-unknown/release/spl_token.so',
    );
    tokenSwapProgramId = await loadProgram(
      connection,
      '../../target/bpfel-unknown-unknown/release/spl_token_swap.so',
    );
    await store.save('config.json', {
      tokenProgramId: tokenProgramId.toString(),
      tokenSwapProgramId: tokenSwapProgramId.toString(),
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
  const payer = await newAccountWithLamports(
    connection,
    100000000000 /* wag */,
  );
  owner = await newAccountWithLamports(connection, 100000000000 /* wag */);
  const tokenSwapAccount = new Account();

  [authority, nonce] = await PublicKey.findProgramAddress(
    [tokenSwapAccount.publicKey.toBuffer()],
    tokenSwapProgramId,
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
  const swapPayer = await newAccountWithLamports(
    connection,
    100000000000 /* wag */,
  );
  tokenSwap = await TokenSwap.createTokenSwap(
    connection,
    swapPayer,
    tokenSwapAccount,
    authority,
    nonce,
    tokenAccountA,
    tokenAccountB,
    tokenPool.publicKey,
    tokenAccountPool,
    tokenSwapProgramId,
    tokenProgramId,
    1,
    4,
  );

  console.log('loading token swap');
  const fetchedTokenSwap = await TokenSwap.loadTokenSwap(
    connection,
    tokenSwapAccount.publicKey,
    tokenSwapProgramId,
    swapPayer,
  );

  assert(fetchedTokenSwap.tokenProgramId.equals(tokenProgramId));
  assert(fetchedTokenSwap.tokenAccountA.equals(tokenAccountA));
  assert(fetchedTokenSwap.tokenAccountB.equals(tokenAccountB));
  assert(fetchedTokenSwap.poolToken.equals(tokenPool.publicKey));
  assert(1 == fetchedTokenSwap.feeNumerator.toNumber());
  assert(4 == fetchedTokenSwap.feeDenominator.toNumber());
}

export async function deposit(): Promise<void> {
  const poolMintInfo = await tokenPool.getMintInfo();
  const supply = poolMintInfo.supply.toNumber();
  const swapTokenA = await mintA.getAccountInfo(tokenAccountA);
  const tokenA = (swapTokenA.amount.toNumber() * POOL_TOKEN_AMOUNT) / supply;
  const swapTokenB = await mintB.getAccountInfo(tokenAccountB);
  const tokenB = (swapTokenB.amount.toNumber() * POOL_TOKEN_AMOUNT) / supply;

  console.log('Creating depositor token a account');
  const userAccountA = await mintA.createAccount(owner.publicKey);
  await mintA.mintTo(userAccountA, owner, [], tokenA);
  await mintA.approve(userAccountA, authority, owner, [], tokenA);
  console.log('Creating depositor token b account');
  const userAccountB = await mintB.createAccount(owner.publicKey);
  await mintB.mintTo(userAccountB, owner, [], tokenB);
  await mintB.approve(userAccountB, authority, owner, [], tokenB);
  console.log('Creating depositor pool token account');
  const newAccountPool = await tokenPool.createAccount(owner.publicKey);

  console.log('Depositing into swap');
  await tokenSwap.deposit(
    userAccountA,
    userAccountB,
    newAccountPool,
    POOL_TOKEN_AMOUNT,
    tokenA,
    tokenB,
  );

  let info;
  info = await mintA.getAccountInfo(userAccountA);
  assert(info.amount.toNumber() == 0);
  info = await mintB.getAccountInfo(userAccountB);
  assert(info.amount.toNumber() == 0);
  info = await mintA.getAccountInfo(tokenAccountA);
  assert(info.amount.toNumber() == BASE_AMOUNT + tokenA);
  info = await mintB.getAccountInfo(tokenAccountB);
  assert(info.amount.toNumber() == BASE_AMOUNT + tokenB);
  info = await tokenPool.getAccountInfo(newAccountPool);
  assert(info.amount.toNumber() == POOL_TOKEN_AMOUNT);
}

export async function withdraw(): Promise<void> {
  const poolMintInfo = await tokenPool.getMintInfo();
  const supply = poolMintInfo.supply.toNumber();
  let swapTokenA = await mintA.getAccountInfo(tokenAccountA);
  let swapTokenB = await mintB.getAccountInfo(tokenAccountB);
  const tokenA = (swapTokenA.amount.toNumber() * POOL_TOKEN_AMOUNT) / supply;
  const tokenB = (swapTokenB.amount.toNumber() * POOL_TOKEN_AMOUNT) / supply;

  console.log('Creating withdraw token A account');
  let userAccountA = await mintA.createAccount(owner.publicKey);
  console.log('Creating withdraw token B account');
  let userAccountB = await mintB.createAccount(owner.publicKey);

  console.log('Approving withdrawal from pool account');
  await tokenPool.approve(
    tokenAccountPool,
    authority,
    owner,
    [],
    POOL_TOKEN_AMOUNT,
  );

  console.log('Withdrawing pool tokens for A and B tokens');
  await tokenSwap.withdraw(
    userAccountA,
    userAccountB,
    tokenAccountPool,
    POOL_TOKEN_AMOUNT,
    tokenA,
    tokenB,
  );

  //const poolMintInfo = await tokenPool.getMintInfo();
  swapTokenA = await mintA.getAccountInfo(tokenAccountA);
  swapTokenB = await mintB.getAccountInfo(tokenAccountB);

  let info = await tokenPool.getAccountInfo(tokenAccountPool);
  assert(
    info.amount.toNumber() == DEFAULT_POOL_TOKEN_AMOUNT - POOL_TOKEN_AMOUNT,
  );
  assert(swapTokenA.amount.toNumber() == BASE_AMOUNT);
  assert(swapTokenB.amount.toNumber() == BASE_AMOUNT);
  info = await mintA.getAccountInfo(userAccountA);
  assert(info.amount.toNumber() == tokenA);
  info = await mintB.getAccountInfo(userAccountB);
  assert(info.amount.toNumber() == tokenB);
}

export async function swap(): Promise<void> {
  console.log('Creating swap token a account');
  let userAccountA = await mintA.createAccount(owner.publicKey);
  await mintA.mintTo(userAccountA, owner, [], SWAP_AMOUNT_IN);
  await mintA.approve(userAccountA, authority, owner, [], SWAP_AMOUNT_IN);
  console.log('Creating swap token b account');
  let userAccountB = await mintB.createAccount(owner.publicKey);

  console.log('Swapping');
  await tokenSwap.swap(
    userAccountA,
    tokenAccountA,
    tokenAccountB,
    userAccountB,
    SWAP_AMOUNT_IN,
    SWAP_AMOUNT_OUT,
  );
  await sleep(500);
  let info;
  info = await mintA.getAccountInfo(userAccountA);
  assert(info.amount.toNumber() == 0);
  info = await mintA.getAccountInfo(tokenAccountA);
  assert(info.amount.toNumber() == BASE_AMOUNT + SWAP_AMOUNT_IN);
  info = await mintB.getAccountInfo(tokenAccountB);
  assert(info.amount.toNumber() == BASE_AMOUNT - SWAP_AMOUNT_OUT);
  info = await mintB.getAccountInfo(userAccountB);
  assert(info.amount.toNumber() == SWAP_AMOUNT_OUT);
  info = await tokenPool.getAccountInfo(tokenAccountPool);
  assert(
    info.amount.toNumber() == DEFAULT_POOL_TOKEN_AMOUNT - POOL_TOKEN_AMOUNT,
  );
}
