// @flow

import fs from 'mz/fs';
import {Connection, BpfLoader, PublicKey} from '@solana/web3.js';
import semver from 'semver';

import {Token, TokenAmount} from '../client/token';
import {url} from '../../url';
import {newAccountWithLamports} from '../client/util/new-account-with-lamports';
import {sleep} from '../client/util/sleep';

// Loaded token program's program id
let programId: PublicKey;

// A token created by the next test and used by all subsequent tests
let testToken: Token;

// Initial owner of the token supply
let initialOwner;
let initialOwnerTokenAccount: PublicKey;

function assert(condition, message) {
  if (!condition) {
    console.log(Error().stack + ':token-test.js');
    throw message || 'Assertion failed';
  }
}

async function didThrow(func, args): Promise<boolean> {
  try {
    await func.apply(args);
  } catch (e) {
    return true;
  }
  return false;
}

let connection;
async function getConnection(): Promise<Connection> {
  if (connection) return connection;

  let newConnection = new Connection(url);
  const version = await newConnection.getVersion();

  // commitment params are only supported >= 0.21.0
  const solanaCoreVersion = version['solana-core'].split(' ')[0];
  if (semver.gte(solanaCoreVersion, '0.21.0')) {
    newConnection = new Connection(url, 'recent');
  }

  // eslint-disable-next-line require-atomic-updates
  connection = newConnection;
  console.log('Connection to cluster established:', url, version);
  return connection;
}

export async function loadTokenProgram(): Promise<void> {
  const NUM_RETRIES = 500; /* allow some number of retries */
  const data = await fs.readFile(
    'src/program/target/bpfel-unknown-unknown/release/spl_token.so',
  );
  const connection = await getConnection();
  const {feeCalculator} = await connection.getRecentBlockhash();
  const balanceNeeded =
    feeCalculator.lamportsPerSignature *
      (BpfLoader.getMinNumSignatures(data.length) + NUM_RETRIES) +
    (await connection.getMinimumBalanceForRentExemption(data.length));

  const from = await newAccountWithLamports(connection, balanceNeeded);
  console.log('Loading Token program...');
  programId = await BpfLoader.load(connection, from, data);
}

export async function createNewToken(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded =
    (await Token.getMinBalanceRentForExemptToken(connection)) +
    (await Token.getMinBalanceRentForExemptTokenAccount(connection));
  initialOwner = await newAccountWithLamports(connection, balanceNeeded);
  [testToken, initialOwnerTokenAccount] = await Token.createNewToken(
    connection,
    initialOwner,
    new TokenAmount(10000),
    2,
    programId,
  );

  const tokenInfo = await testToken.tokenInfo();
  assert(tokenInfo.supply.toNumber() == 10000);
  assert(tokenInfo.decimals == 2);

  const accountInfo = await testToken.accountInfo(initialOwnerTokenAccount);
  assert(accountInfo.token.equals(testToken.token));
  assert(accountInfo.owner.equals(initialOwner.publicKey));
  assert(accountInfo.amount.toNumber() == 10000);
  assert(accountInfo.source == null);
  assert(accountInfo.originalAmount.toNumber() == 0);
}

export async function createNewTokenAccount(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded = await Token.getMinBalanceRentForExemptTokenAccount(
    connection,
  );
  const destOwner = await newAccountWithLamports(connection, balanceNeeded);
  const dest = await testToken.newAccount(destOwner);
  const accountInfo = await testToken.accountInfo(dest);
  assert(accountInfo.token.equals(testToken.token));
  assert(accountInfo.owner.equals(destOwner.publicKey));
  assert(accountInfo.amount.toNumber() == 0);
  assert(accountInfo.source == null);
}

export async function transfer(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded = await Token.getMinBalanceRentForExemptTokenAccount(
    connection,
  );
  const destOwner = await newAccountWithLamports(connection, balanceNeeded);
  const dest = await testToken.newAccount(destOwner);

  await testToken.transfer(initialOwner, initialOwnerTokenAccount, dest, 123);
  await sleep(500);

  const destAccountInfo = await testToken.accountInfo(dest);
  assert(destAccountInfo.amount.toNumber() == 123);
}

export async function approveRevoke(): Promise<void> {
  if (programId == null) {
    console.log('test skipped, requires "load token program" to succeed');
    return;
  }

  const connection = await getConnection();
  const balanceNeeded = await Token.getMinBalanceRentForExemptTokenAccount(
    connection,
  );
  const delegateOwner = await newAccountWithLamports(connection, balanceNeeded);
  const delegate = await testToken.newAccount(
    delegateOwner,
    initialOwnerTokenAccount,
  );

  await testToken.approve(
    initialOwner,
    initialOwnerTokenAccount,
    delegate,
    456,
  );

  let delegateAccountInfo = await testToken.accountInfo(delegate);
  assert(delegateAccountInfo.amount.toNumber() == 456);
  assert(delegateAccountInfo.originalAmount.toNumber() == 456);
  if (delegateAccountInfo.source === null) {
    throw new Error('source should not be null');
  } else {
    assert(delegateAccountInfo.source.equals(initialOwnerTokenAccount));
  }

  await testToken.revoke(initialOwner, initialOwnerTokenAccount, delegate);
  delegateAccountInfo = await testToken.accountInfo(delegate);
  assert(delegateAccountInfo.amount.toNumber() == 0);
  assert(delegateAccountInfo.originalAmount.toNumber() == 0);
  if (delegateAccountInfo.source === null) {
    throw new Error('source should not be null');
  } else {
    assert(delegateAccountInfo.source.equals(initialOwnerTokenAccount));
  }
}

export async function invalidApprove(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded =
    (await Token.getMinBalanceRentForExemptTokenAccount(connection)) * 3;
  const owner = await newAccountWithLamports(connection, balanceNeeded);
  const account1 = await testToken.newAccount(owner);
  const account1Delegate = await testToken.newAccount(owner, account1);
  const account2 = await testToken.newAccount(owner);

  // account2 is not a delegate account of account1
  assert(didThrow(testToken.approve, [owner, account1, account2, 123]));
  // account1Delegate is not a delegate account of account2
  assert(didThrow(testToken.approve, [owner, account2, account1Delegate, 123]));
}

export async function failOnApproveOverspend(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded =
    (await Token.getMinBalanceRentForExemptTokenAccount(connection)) * 3;
  const owner = await newAccountWithLamports(connection, balanceNeeded);
  const account1 = await testToken.newAccount(owner);
  const account1Delegate = await testToken.newAccount(owner, account1);
  const account2 = await testToken.newAccount(owner);

  await testToken.transfer(
    initialOwner,
    initialOwnerTokenAccount,
    account1,
    10,
  );

  await testToken.approve(owner, account1, account1Delegate, 2);

  let delegateAccountInfo = await testToken.accountInfo(account1Delegate);
  assert(delegateAccountInfo.amount.toNumber() == 2);
  assert(delegateAccountInfo.originalAmount.toNumber() == 2);

  await testToken.transfer(owner, account1Delegate, account2, 1);

  delegateAccountInfo = await testToken.accountInfo(account1Delegate);
  assert(delegateAccountInfo.amount.toNumber() == 1);
  assert(delegateAccountInfo.originalAmount.toNumber() == 2);

  await testToken.transfer(owner, account1Delegate, account2, 1);

  delegateAccountInfo = await testToken.accountInfo(account1Delegate);
  assert(delegateAccountInfo.amount.toNumber() == 0);
  assert(delegateAccountInfo.originalAmount.toNumber() == 2);

  assert(didThrow(testToken.transfer, [owner, account1Delegate, account2, 1]));
}

export async function setOwner(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded = await Token.getMinBalanceRentForExemptTokenAccount(
    connection,
  );
  const owner = await newAccountWithLamports(connection, balanceNeeded);
  const newOwner = await newAccountWithLamports(connection, balanceNeeded);
  const account = await testToken.newAccount(owner);

  await testToken.setOwner(owner, account, newOwner.publicKey);
  assert(didThrow(testToken.setOwner, [owner, account, newOwner.publicKey]));
  await testToken.setOwner(newOwner, account, owner.publicKey);
}
