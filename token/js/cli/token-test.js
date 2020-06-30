// @flow

import fs from 'mz/fs';
import {Account, Connection, BpfLoader, PublicKey} from '@solana/web3.js';
import semver from 'semver';

import {Token, TokenAmount} from '../client/token';
import {url} from '../url';
import {newAccountWithLamports} from '../client/util/new-account-with-lamports';
import {sleep} from '../client/util/sleep';

// Loaded token program's program id
let programId: PublicKey;

// A token created by the next test and used by all subsequent tests
let testToken: Token;

// Initial owner of the token supply
let tokenOwner: Account;

// Initial token account
let testAccountOwner: Account;
let testAccount: PublicKey;

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

  let newConnection = new Connection(url, 'recent', );
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

export async function loadTokenProgram(path: string): Promise<PublicKey> {
  const NUM_RETRIES = 500; /* allow some number of retries */
  const data = await fs.readFile(path
  );
  const connection = await getConnection();
  const {feeCalculator} = await connection.getRecentBlockhash();
  const balanceNeeded =
    feeCalculator.lamportsPerSignature *
      (BpfLoader.getMinNumSignatures(data.length) + NUM_RETRIES) +
    (await connection.getMinimumBalanceForRentExemption(data.length));

  const from = await newAccountWithLamports(connection, balanceNeeded);
  const program_account = new Account();
  console.log('Loading Token program...');
  await BpfLoader.load(connection, from, program_account, data);
  programId = program_account.publicKey;
  return programId;
}

export async function createNewToken(): Promise<void> {
  const connection = await getConnection();
  const payer = await newAccountWithLamports(connection, 100000000000 /* wag */);
  tokenOwner = new Account();
  testAccountOwner = new Account();
  [testToken, testAccount] = await Token.createNewToken(
    connection,
    payer,
    tokenOwner.publicKey,
    testAccountOwner.publicKey,
    new TokenAmount(10000),
    2,
    programId,
    false,
  );

  const tokenInfo = await testToken.getTokenInfo();
  assert(tokenInfo.supply.toNumber() == 10000);
  assert(tokenInfo.decimals == 2);
  assert(tokenInfo.owner == null);

  const accountInfo = await testToken.getAccountInfo(testAccount);
  assert(accountInfo.token.equals(testToken.publicKey));
  assert(accountInfo.owner.equals(testAccountOwner.publicKey));
  assert(accountInfo.amount.toNumber() == 10000);
  assert(accountInfo.source == null);
  assert(accountInfo.originalAmount.toNumber() == 0);
}

export async function createNewAccount(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(
    connection,
  );
  const destOwner = await newAccountWithLamports(connection, balanceNeeded);
  const dest = await testToken.newAccount(destOwner.publicKey);
  const accountInfo = await testToken.getAccountInfo(dest);
  assert(accountInfo.token.equals(testToken.publicKey));
  assert(accountInfo.owner.equals(destOwner.publicKey));
  assert(accountInfo.amount.toNumber() == 0);
  assert(accountInfo.source == null);
}

export async function transfer(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(
    connection,
  );
  const destOwner = await newAccountWithLamports(connection, balanceNeeded);
  const dest = await testToken.newAccount(destOwner.publicKey);

  await testToken.transfer(testAccountOwner, testAccount, dest, 123);
  await sleep(500);

  let destTokenAccountInfo = await testToken.getAccountInfo(dest);
  assert(destTokenAccountInfo.amount.toNumber() == 123);
}

export async function approveRevoke(): Promise<void> {
  if (programId == null) {
    console.log('test skipped, requires "load token program" to succeed');
    return;
  }

  const connection = await getConnection();
  const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(
    connection,
  );
  const delegateOwner = await newAccountWithLamports(connection, balanceNeeded);
  const delegate = await testToken.newAccount(
    delegateOwner.publicKey,
    testAccount,
  );

  await testToken.approve(
    testAccountOwner,
    testAccount,
    delegate,
    456,
  );

  let delegateAccountInfo = await testToken.getAccountInfo(delegate);
  assert(delegateAccountInfo.amount.toNumber() == 456);
  assert(delegateAccountInfo.originalAmount.toNumber() == 456);
  if (delegateAccountInfo.source === null) {
    throw new Error('source should not be null');
  } else {
    assert(delegateAccountInfo.source.equals(testAccount));
  }

  await testToken.revoke(testAccountOwner, testAccount, delegate);
  delegateAccountInfo = await testToken.getAccountInfo(delegate);
  assert(delegateAccountInfo.amount.toNumber() == 0);
  assert(delegateAccountInfo.originalAmount.toNumber() == 0);
  if (delegateAccountInfo.source === null) {
    throw new Error('source should not be null');
  } else {
    assert(delegateAccountInfo.source.equals(testAccount));
  }
}

export async function invalidApprove(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded =
    (await Token.getMinBalanceRentForExemptAccount(connection)) * 3;
  const owner = await newAccountWithLamports(connection, balanceNeeded);
  const account1 = await testToken.newAccount(owner.publicKey);
  const account1Delegate = await testToken.newAccount(owner.publicKey, account1);
  const account2 = await testToken.newAccount(owner.publicKey);

  // account2 is not a delegate account of account1
  assert(didThrow(testToken.approve, [owner, account1, account2, 123]));
  // account1Delegate is not a delegate account of account2
  assert(didThrow(testToken.approve, [owner, account2, account1Delegate, 123]));
}

export async function failOnApproveOverspend(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded =
    (await Token.getMinBalanceRentForExemptAccount(connection)) * 3;
  const owner = await newAccountWithLamports(connection, balanceNeeded);
  const account1 = await testToken.newAccount(owner.publicKey);
  const account1Delegate = await testToken.newAccount(owner.publicKey, account1);
  const account2 = await testToken.newAccount(owner.publicKey);

  await testToken.transfer(
    testAccountOwner,
    testAccount,
    account1,
    10,
  );

  await testToken.approve(owner, account1, account1Delegate, 2);

  let delegateAccountInfo = await testToken.getAccountInfo(account1Delegate);
  assert(delegateAccountInfo.amount.toNumber() == 2);
  assert(delegateAccountInfo.originalAmount.toNumber() == 2);

  await testToken.transfer(owner, account1Delegate, account2, 1);

  delegateAccountInfo = await testToken.getAccountInfo(account1Delegate);
  assert(delegateAccountInfo.amount.toNumber() == 1);
  assert(delegateAccountInfo.originalAmount.toNumber() == 2);

  await testToken.transfer(owner, account1Delegate, account2, 1);

  delegateAccountInfo = await testToken.getAccountInfo(account1Delegate);
  assert(delegateAccountInfo.amount.toNumber() == 0);
  assert(delegateAccountInfo.originalAmount.toNumber() == 2);

  assert(didThrow(testToken.transfer, [owner, account1Delegate, account2, 1]));
}

export async function setOwner(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(
    connection,
  );
  const owner = await newAccountWithLamports(connection, balanceNeeded);
  const newOwner = await newAccountWithLamports(connection, balanceNeeded);
  const owned = await testToken.newAccount(owner.publicKey);

  await testToken.setOwner(owner, owned, newOwner.publicKey);
  assert(didThrow(testToken.setOwner, [owner, owned, newOwner.publicKey]));
  await testToken.setOwner(newOwner, owned, owner.publicKey);
}

export async function mintTo(): Promise<void> {
  const connection = await getConnection();
  const payer = await newAccountWithLamports(connection, 100000000000 /* wag */);
  const tokenOwner = new Account();
  const testAccountOwner = new Account();
  const [mintableToken, initialAccount] = await Token.createNewToken(
    connection,
    payer,
    tokenOwner.publicKey,
    testAccountOwner.publicKey,
    new TokenAmount(10000),
    2,
    programId,
    true,
  );

  {
    const tokenInfo = await mintableToken.getTokenInfo();
    assert(tokenInfo.supply.toNumber() == 10000);
    assert(tokenInfo.decimals == 2);
    if (tokenInfo.owner === null) {
      throw new Error('owner should not be null');
    } else {
      assert(tokenInfo.owner.equals(tokenOwner.publicKey));
    }

    const accountInfo = await mintableToken.getAccountInfo(initialAccount);
    assert(accountInfo.token.equals(mintableToken.publicKey));
    assert(accountInfo.owner.equals(testAccountOwner.publicKey));
    assert(accountInfo.amount.toNumber() == 10000);
    assert(accountInfo.source == null);
    assert(accountInfo.originalAmount.toNumber() == 0);
  }

  const dest = await mintableToken.newAccount(testAccountOwner.publicKey);
  await mintableToken.mintTo(tokenOwner, dest, 42);

  {
    const tokenInfo = await mintableToken.getTokenInfo();
    assert(tokenInfo.supply.toNumber() == 10042);
    assert(tokenInfo.decimals == 2);
    if (tokenInfo.owner === null) {
      throw new Error('owner should not be null');
    } else {
      assert(tokenInfo.owner.equals(tokenOwner.publicKey));
    }

    const accountInfo = await mintableToken.getAccountInfo(dest);
    assert(accountInfo.token.equals(mintableToken.publicKey));
    assert(accountInfo.owner.equals(testAccountOwner.publicKey));
    assert(accountInfo.amount.toNumber() == 42);
    assert(accountInfo.source == null);
    assert(accountInfo.originalAmount.toNumber() == 0);
  }
}

export async function burn(): Promise<void> {
  let tokenInfo = await testToken.getTokenInfo();
  const supply = tokenInfo.supply.toNumber();
  let accountInfo = await testToken.getAccountInfo(testAccount);
  const amount = accountInfo.amount.toNumber();

  await testToken.burn(testAccountOwner, testAccount, 1);
  await sleep(500);

  tokenInfo = await testToken.getTokenInfo();
  assert(tokenInfo.supply.toNumber() == supply - 1);
  accountInfo = await testToken.getAccountInfo(testAccount);
  assert(accountInfo.amount.toNumber() == amount - 1);
}
