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

export async function loadTokenProgram(): Promise<void> {
  const NUM_RETRIES = 500; /* allow some number of retries */
  const data = await fs.readFile(
    '../target/bpfel-unknown-unknown/release/spl_token.so',
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
}

export async function createNewToken(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded =
    (await Token.getMinBalanceRentForExemptToken(connection)) +
    (await Token.getMinBalanceRentForExemptAccount(connection));
  initialOwner = await newAccountWithLamports(connection, balanceNeeded);
  [testToken, initialOwnerTokenAccount] = await Token.createNewToken(
    connection,
    initialOwner,
    new TokenAmount(10000),
    2,
    programId,
    false,
  );

  const TokenInfo = await testToken.TokenInfo();
  assert(TokenInfo.supply.toNumber() == 10000);
  assert(TokenInfo.decimals == 2);
  assert(TokenInfo.owner == null);

  const TokenAccountInfo = await testToken.TokenAccountInfo(initialOwnerTokenAccount);
  assert(TokenAccountInfo.token.equals(testToken.token));
  assert(TokenAccountInfo.owner.equals(initialOwner.publicKey));
  assert(TokenAccountInfo.amount.toNumber() == 10000);
  assert(TokenAccountInfo.source == null);
  assert(TokenAccountInfo.originalAmount.toNumber() == 0);
}

export async function createNewAccount(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(
    connection,
  );
  const destOwner = await newAccountWithLamports(connection, balanceNeeded);
  const dest = await testToken.newAccount(destOwner);
  const TokenAccountInfo = await testToken.TokenAccountInfo(dest);
  assert(TokenAccountInfo.token.equals(testToken.token));
  assert(TokenAccountInfo.owner.equals(destOwner.publicKey));
  assert(TokenAccountInfo.amount.toNumber() == 0);
  assert(TokenAccountInfo.source == null);
}

export async function transfer(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(
    connection,
  );
  const destOwner = await newAccountWithLamports(connection, balanceNeeded);
  const dest = await testToken.newAccount(destOwner);

  await testToken.transfer(initialOwner, initialOwnerTokenAccount, dest, 123);
  await sleep(500);

  let destTokenAccountInfo = await testToken.TokenAccountInfo(dest);
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
    delegateOwner,
    initialOwnerTokenAccount,
  );

  await testToken.approve(
    initialOwner,
    initialOwnerTokenAccount,
    delegate,
    456,
  );

  let delegateTokenAccountInfo = await testToken.TokenAccountInfo(delegate);
  assert(delegateTokenAccountInfo.amount.toNumber() == 456);
  assert(delegateTokenAccountInfo.originalAmount.toNumber() == 456);
  if (delegateTokenAccountInfo.source === null) {
    throw new Error('source should not be null');
  } else {
    assert(delegateTokenAccountInfo.source.equals(initialOwnerTokenAccount));
  }

  await testToken.revoke(initialOwner, initialOwnerTokenAccount, delegate);
  delegateTokenAccountInfo = await testToken.TokenAccountInfo(delegate);
  assert(delegateTokenAccountInfo.amount.toNumber() == 0);
  assert(delegateTokenAccountInfo.originalAmount.toNumber() == 0);
  if (delegateTokenAccountInfo.source === null) {
    throw new Error('source should not be null');
  } else {
    assert(delegateTokenAccountInfo.source.equals(initialOwnerTokenAccount));
  }
}

export async function invalidApprove(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded =
    (await Token.getMinBalanceRentForExemptAccount(connection)) * 3;
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
    (await Token.getMinBalanceRentForExemptAccount(connection)) * 3;
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

  let delegateTokenAccountInfo = await testToken.TokenAccountInfo(account1Delegate);
  assert(delegateTokenAccountInfo.amount.toNumber() == 2);
  assert(delegateTokenAccountInfo.originalAmount.toNumber() == 2);

  await testToken.transfer(owner, account1Delegate, account2, 1);

  delegateTokenAccountInfo = await testToken.TokenAccountInfo(account1Delegate);
  assert(delegateTokenAccountInfo.amount.toNumber() == 1);
  assert(delegateTokenAccountInfo.originalAmount.toNumber() == 2);

  await testToken.transfer(owner, account1Delegate, account2, 1);

  delegateTokenAccountInfo = await testToken.TokenAccountInfo(account1Delegate);
  assert(delegateTokenAccountInfo.amount.toNumber() == 0);
  assert(delegateTokenAccountInfo.originalAmount.toNumber() == 2);

  assert(didThrow(testToken.transfer, [owner, account1Delegate, account2, 1]));
}

export async function setOwner(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded = await Token.getMinBalanceRentForExemptAccount(
    connection,
  );
  const owner = await newAccountWithLamports(connection, balanceNeeded);
  const newOwner = await newAccountWithLamports(connection, balanceNeeded);
  const owned = await testToken.newAccount(owner);

  await testToken.setOwner(owner, owned, newOwner.publicKey);
  assert(didThrow(testToken.setOwner, [owner, owned, newOwner.publicKey]));
  await testToken.setOwner(newOwner, owned, owner.publicKey);
}

export async function mintTo(): Promise<void> {
  const connection = await getConnection();
  const balanceNeeded =
    (await Token.getMinBalanceRentForExemptToken(connection)) +
    (await Token.getMinBalanceRentForExemptAccount(connection)) * 2;

  const owner = await newAccountWithLamports(connection, balanceNeeded);

  const [mintableToken, initialAccount] = await Token.createNewToken(
    connection,
    owner,
    new TokenAmount(10000),
    2,
    programId,
    true,
  );

  {
    const TokenInfo = await mintableToken.TokenInfo();
    assert(TokenInfo.supply.toNumber() == 10000);
    assert(TokenInfo.decimals == 2);
    if (TokenInfo.owner === null) {
      throw new Error('owner should not be null');
    } else {
      assert(TokenInfo.owner.equals(owner.publicKey));
    }

    const TokenAccountInfo = await mintableToken.TokenAccountInfo(initialAccount);
    assert(TokenAccountInfo.token.equals(mintableToken.token));
    assert(TokenAccountInfo.owner.equals(owner.publicKey));
    assert(TokenAccountInfo.amount.toNumber() == 10000);
    assert(TokenAccountInfo.source == null);
    assert(TokenAccountInfo.originalAmount.toNumber() == 0);
  }

  const dest = await mintableToken.newAccount(owner);
  await mintableToken.mintTo(owner, mintableToken.token, dest, 42);

  {
    const TokenInfo = await mintableToken.TokenInfo();
    assert(TokenInfo.supply.toNumber() == 10042);
    assert(TokenInfo.decimals == 2);
    if (TokenInfo.owner === null) {
      throw new Error('owner should not be null');
    } else {
      assert(TokenInfo.owner.equals(owner.publicKey));
    }

    const TokenAccountInfo = await mintableToken.TokenAccountInfo(dest);
    assert(TokenAccountInfo.token.equals(mintableToken.token));
    assert(TokenAccountInfo.owner.equals(owner.publicKey));
    assert(TokenAccountInfo.amount.toNumber() == 42);
    assert(TokenAccountInfo.source == null);
    assert(TokenAccountInfo.originalAmount.toNumber() == 0);
  }
}

export async function burn(): Promise<void> {
  let TokenInfo = await testToken.TokenInfo();
  const supply = TokenInfo.supply.toNumber();
  let TokenAccountInfo = await testToken.TokenAccountInfo(initialOwnerTokenAccount);
  const amount = TokenAccountInfo.amount.toNumber();

  await testToken.burn(initialOwner, initialOwnerTokenAccount, 1);
  await sleep(500);

  TokenInfo = await testToken.TokenInfo();
  assert(TokenInfo.supply.toNumber() == supply - 1);
  TokenAccountInfo = await testToken.TokenAccountInfo(initialOwnerTokenAccount);
  assert(TokenAccountInfo.amount.toNumber() == amount - 1);
}
