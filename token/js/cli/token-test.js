// @flow

import fs from 'mz/fs';
import {Account, Connection, BpfLoader, PublicKey} from '@solana/web3.js';
import semver from 'semver';

import {Token, TokenAmount} from '../client/token';
import {url} from '../url';
import {newAccountWithLamports} from '../client/util/new-account-with-lamports';
import {sleep} from '../client/util/sleep';
import {Store} from '../client/util/store';

// Loaded token program's program id
let programId: PublicKey;

// A token created by the next test and used by all subsequent tests
let testToken: Token;

// Initial owner of the mint
let mintOwner: Account;

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

async function GetPrograms(connection: Connection): Promise<PublicKey> {
  const store = new Store();
  let tokenProgramId = null;
  try {
    const config = await store.load('config.json');
    console.log('Using pre-loaded Token program');
    console.log('  Note: To reload program remove client/util/sore/config.json');
    tokenProgramId = new PublicKey(config.tokenProgramId);
  } catch (err) {
    tokenProgramId = await loadProgram(connection, '../target/bpfel-unknown-unknown/release/spl_token.so');
    await store.save('config.json', {
      tokenProgramId: tokenProgramId.toString(),
    });
  }
  return tokenProgramId;
}

export async function loadTokenProgram(): Promise<void> {
  const connection = await getConnection();
  programId = await GetPrograms(connection);

  console.log('Token Program ID', programId.toString());
}

export async function createMint(): Promise<void> {
  const connection = await getConnection();
  const payer = await newAccountWithLamports(connection, 100000000000 /* wag */);
  mintOwner = new Account();
  testAccountOwner = new Account();
  [testToken, testAccount] = await Token.createMint(
    connection,
    payer,
    mintOwner.publicKey,
    testAccountOwner.publicKey,
    new TokenAmount(10000),
    2,
    programId,
    false,
  );

  const mintInfo = await testToken.getMintInfo();
  assert(mintInfo.supply.toNumber() == 10000);
  assert(mintInfo.decimals == 2);
  assert(mintInfo.owner == null);

  const accountInfo = await testToken.getAccountInfo(testAccount);
  assert(accountInfo.mint.equals(testToken.publicKey));
  assert(accountInfo.owner.equals(testAccountOwner.publicKey));
  assert(accountInfo.amount.toNumber() == 10000);
  assert(accountInfo.delegate == null);
  assert(accountInfo.delegatedAmount.toNumber() == 0);
}

export async function createAccount(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(
    connection,
  );
  const destOwner = await newAccountWithLamports(connection, balanceNeeded);
  const dest = await testToken.createAccount(destOwner.publicKey);
  const accountInfo = await testToken.getAccountInfo(dest);
  assert(accountInfo.mint.equals(testToken.publicKey));
  assert(accountInfo.owner.equals(destOwner.publicKey));
  assert(accountInfo.amount.toNumber() == 0);
  assert(accountInfo.delegate == null);
}

export async function transfer(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(
    connection,
  );
  const destOwner = await newAccountWithLamports(connection, balanceNeeded);
  const dest = await testToken.createAccount(destOwner.publicKey);

  await testToken.transfer(testAccount, dest,testAccountOwner, 123);
  await sleep(500);

  let destAccountInfo = await testToken.getAccountInfo(dest);
  assert(destAccountInfo.amount.toNumber() == 123);
}

export async function approveRevoke(): Promise<void> {
  if (programId == null) {
    console.log('test skipped, requires "load token program" to succeed');
    return;
  }

  const delegate = new PublicKey();
  await testToken.approve(
    testAccount,
    delegate,
    testAccountOwner,
    456,
  );
  let testAccountInfo = await testToken.getAccountInfo(testAccount);
  assert(testAccountInfo.delegatedAmount.toNumber() == 456);
  if (testAccountInfo.delegate === null) {
    throw new Error('deleage should not be null');
  } else {
    assert(testAccountInfo.delegate.equals(delegate));
  }

  await testToken.revoke(testAccount, delegate, testAccountOwner);
  testAccountInfo = await testToken.getAccountInfo(testAccount);
  assert(testAccountInfo.delegatedAmount.toNumber() == 0);
  if (testAccountInfo.delegate != null) {
    throw new Error('delegate should be null');
  }
}

export async function invalidApprove(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded =
    (await Token.getMinBalanceRentForExemptAccount(connection)) * 3;
  const owner = await newAccountWithLamports(connection, balanceNeeded);
  const account1 = await testToken.createAccount(owner.publicKey);
  const account2 = await testToken.createAccount(owner.publicKey);
  const delegate = new Account();

  // account2 is not a delegate account of account1
  assert(didThrow(testToken.approve, [account1, account2, owner, 123]));
  // account1Delegate is not a delegate account of account2
  assert(didThrow(testToken.approve, [account2, delegate, owner, 123]));
}

export async function failOnApproveOverspend(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded =
    (await Token.getMinBalanceRentForExemptAccount(connection)) * 3;
  const owner = await newAccountWithLamports(connection, balanceNeeded);
  const account1 = await testToken.createAccount(owner.publicKey);
  const account2 = await testToken.createAccount(owner.publicKey);
  const delegate = new Account();

  await testToken.transfer(
    testAccount,
    account1,
    testAccountOwner,
    10,
  );

  await testToken.approve(account1, delegate.publicKey, owner, 2);

  let account1Info = await testToken.getAccountInfo(account1);
  assert(account1Info.amount.toNumber() == 10);
  assert(account1Info.delegatedAmount.toNumber() == 2);
  if (account1Info.delegate === null) {
    throw new Error('deleage should not be null');
  } else {
    assert(account1Info.delegate.equals(delegate.publicKey));
  }

  await testToken.transfer(account1, account2, delegate, 1);

  account1Info = await testToken.getAccountInfo(account1);
  assert(account1Info.amount.toNumber() == 9);
  assert(account1Info.delegatedAmount.toNumber() == 1);

  await testToken.transfer(account1, account2, delegate, 1);

  account1Info = await testToken.getAccountInfo(account1);
  assert(account1Info.amount.toNumber() == 8);
  assert(account1Info.delegate === null);
  assert(account1Info.delegatedAmount.toNumber() == 0);

  assert(didThrow(testToken.transfer, [account1, account2, delegate, 1]));
}

export async function setOwner(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(
    connection,
  );
  const owner = await newAccountWithLamports(connection, balanceNeeded);
  const newOwner = await newAccountWithLamports(connection, balanceNeeded);
  const owned = await testToken.createAccount(owner.publicKey);

  await testToken.setOwner(owned, newOwner.publicKey, owner);
  assert(didThrow(testToken.setOwner, [owned, newOwner.publicKey, owner]));
  await testToken.setOwner(owned, owner.publicKey,newOwner);
}

export async function mintTo(): Promise<void> {
  const connection = await getConnection();
  const payer = await newAccountWithLamports(connection, 100000000000 /* wag */);
  const mintOwner = new Account();
  const testAccountOwner = new Account();
  const [mintableToken, initialAccount] = await Token.createMint(
    connection,
    payer,
    mintOwner.publicKey,
    testAccountOwner.publicKey,
    new TokenAmount(10000),
    2,
    programId,
    true,
  );

  {
    const mintInfo = await mintableToken.getMintInfo();
    assert(mintInfo.supply.toNumber() == 10000);
    assert(mintInfo.decimals == 2);
    if (mintInfo.owner === null) {
      throw new Error('owner should not be null');
    } else {
      assert(mintInfo.owner.equals(mintOwner.publicKey));
    }

    const accountInfo = await mintableToken.getAccountInfo(initialAccount);
    assert(accountInfo.mint.equals(mintableToken.publicKey));
    assert(accountInfo.owner.equals(testAccountOwner.publicKey));
    assert(accountInfo.amount.toNumber() == 10000);
    assert(accountInfo.delegate == null);
    assert(accountInfo.delegatedAmount.toNumber() == 0);
  }

  const dest = await mintableToken.createAccount(testAccountOwner.publicKey);
  await mintableToken.mintTo(dest, mintOwner,42);

  {
    const mintInfo = await mintableToken.getMintInfo();
    assert(mintInfo.supply.toNumber() == 10042);
    assert(mintInfo.decimals == 2);
    if (mintInfo.owner === null) {
      throw new Error('owner should not be null');
    } else {
      assert(mintInfo.owner.equals(mintOwner.publicKey));
    }

    const accountInfo = await mintableToken.getAccountInfo(dest);
    assert(accountInfo.mint.equals(mintableToken.publicKey));
    assert(accountInfo.owner.equals(testAccountOwner.publicKey));
    assert(accountInfo.amount.toNumber() == 42);
    assert(accountInfo.delegate == null);
    assert(accountInfo.delegatedAmount.toNumber() == 0);
  }
}

export async function burn(): Promise<void> {
  let mintInfo = await testToken.getMintInfo();
  const supply = mintInfo.supply.toNumber();
  let accountInfo = await testToken.getAccountInfo(testAccount);
  const amount = accountInfo.amount.toNumber();

  await testToken.burn(testAccount, testAccountOwner, 1);
  await sleep(500);

  mintInfo = await testToken.getMintInfo();
  assert(mintInfo.supply.toNumber() == supply - 1);
  accountInfo = await testToken.getAccountInfo(testAccount);
  assert(accountInfo.amount.toNumber() == amount - 1);
}
