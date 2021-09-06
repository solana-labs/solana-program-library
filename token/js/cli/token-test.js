// @flow

import fs from 'mz/fs';
import {
  Keypair,
  Connection,
  BpfLoader,
  PublicKey,
  Signer,
  SystemProgram,
  Transaction,
  BPF_LOADER_PROGRAM_ID,
} from '@solana/web3.js';

import {
  Token,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  NATIVE_MINT,
} from '../client/token';
import {url} from '../url';
import {newAccountWithLamports} from '../client/util/new-account-with-lamports';
import {sendAndConfirmTransaction} from '../client/util/send-and-confirm-transaction';
import {sleep} from '../client/util/sleep';
import {Store} from './store';

// Loaded token program's program id
let programId: PublicKey;
let associatedProgramId: PublicKey;

// Accounts setup in createMint and used by all subsequent tests
let testMintAuthority: Signer;
let testToken: Token;
let testTokenDecimals: number = 2;

// Accounts setup in createAccount and used by all subsequent tests
let testAccountOwner: Signer;
let testAccount: PublicKey;

function assert(condition, message) {
  if (!condition) {
    console.log(Error().stack + ':token-test.js');
    throw message || 'Assertion failed';
  }
}

async function didThrow(func): Promise<boolean> {
  try {
    await func();
  } catch (e) {
    return true;
  }
  return false;
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
  const program_account = Keypair.generate();
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

async function GetPrograms(connection: Connection): Promise<void> {
  const programVersion = process.env.PROGRAM_VERSION;
  if (programVersion) {
    switch (programVersion) {
      case '2.0.4':
        programId = TOKEN_PROGRAM_ID;
        associatedProgramId = ASSOCIATED_TOKEN_PROGRAM_ID;
        return;
      default:
        throw new Error('Unknown program version');
    }
  }

  const store = new Store();
  try {
    const config = await store.load('config.json');
    console.log('Using pre-loaded Token programs');
    console.log(
      `  Note: To reload program remove ${Store.getFilename('config.json')}`,
    );
    programId = new PublicKey(config.tokenProgramId);
    associatedProgramId = new PublicKey(config.associatedTokenProgramId);
    let info;
    info = await connection.getAccountInfo(programId);
    assert(info != null);
    info = await connection.getAccountInfo(associatedProgramId);
    assert(info != null);
  } catch (err) {
    console.log(
      'Checking pre-loaded Token programs failed, will load new programs:',
    );
    console.log({err});

    programId = await loadProgram(
      connection,
      '../../target/deploy/spl_token.so',
    );
    associatedProgramId = await loadProgram(
      connection,
      '../../target/deploy/spl_associated_token_account.so',
    );
    await store.save('config.json', {
      tokenProgramId: programId.toString(),
      associatedTokenProgramId: associatedProgramId.toString(),
    });
  }
}

export async function loadTokenProgram(): Promise<void> {
  const connection = await getConnection();
  await GetPrograms(connection);

  console.log('Token Program ID', programId.toString());
  console.log('Associated Token Program ID', associatedProgramId.toString());
}

export async function createMint(): Promise<void> {
  const connection = await getConnection();
  const payer = await newAccountWithLamports(connection, 1000000000 /* wag */);
  testMintAuthority = Keypair.generate();
  testToken = await Token.createMint(
    connection,
    payer,
    testMintAuthority.publicKey,
    testMintAuthority.publicKey,
    testTokenDecimals,
    programId,
  );
  // HACK: override hard-coded ASSOCIATED_TOKEN_PROGRAM_ID with corresponding
  // custom test fixture
  testToken.associatedProgramId = associatedProgramId;

  const mintInfo = await testToken.getMintInfo();
  if (mintInfo.mintAuthority !== null) {
    assert(mintInfo.mintAuthority.equals(testMintAuthority.publicKey));
  } else {
    assert(mintInfo.mintAuthority !== null);
  }
  assert(mintInfo.supply.toNumber() === 0);
  assert(mintInfo.decimals === testTokenDecimals);
  assert(mintInfo.isInitialized === true);
  if (mintInfo.freezeAuthority !== null) {
    assert(mintInfo.freezeAuthority.equals(testMintAuthority.publicKey));
  } else {
    assert(mintInfo.freezeAuthority !== null);
  }
}

export async function createAccount(): Promise<void> {
  testAccountOwner = Keypair.generate();
  testAccount = await testToken.createAccount(testAccountOwner.publicKey);
  const accountInfo = await testToken.getAccountInfo(testAccount);
  assert(accountInfo.mint.equals(testToken.publicKey));
  assert(accountInfo.owner.equals(testAccountOwner.publicKey));
  assert(accountInfo.amount.toNumber() === 0);
  assert(accountInfo.delegate === null);
  assert(accountInfo.delegatedAmount.toNumber() === 0);
  assert(accountInfo.isInitialized === true);
  assert(accountInfo.isFrozen === false);
  assert(accountInfo.isNative === false);
  assert(accountInfo.rentExemptReserve === null);
  assert(accountInfo.closeAuthority === null);

  // you can create as many accounts as with same owner
  const testAccount2 = await testToken.createAccount(
    testAccountOwner.publicKey,
  );
  assert(!testAccount2.equals(testAccount));
}

export async function createAssociatedAccount(): Promise<void> {
  let info;
  const connection = await getConnection();

  const owner = Keypair.generate();
  const associatedAddress = await Token.getAssociatedTokenAddress(
    associatedProgramId,
    programId,
    testToken.publicKey,
    owner.publicKey,
  );

  // associated account shouldn't exist
  info = await connection.getAccountInfo(associatedAddress);
  assert(info == null);

  const createdAddress = await testToken.createAssociatedTokenAccount(
    owner.publicKey,
  );
  assert(createdAddress.equals(associatedAddress));

  // associated account should exist now
  info = await testToken.getAccountInfo(associatedAddress);
  assert(info != null);
  assert(info.mint.equals(testToken.publicKey));
  assert(info.owner.equals(owner.publicKey));
  assert(info.amount.toNumber() === 0);

  // creating again should cause TX error for the associated token account
  assert(
    await didThrow(async () => {
      await testToken.createAssociatedTokenAccount(owner.publicKey);
    }),
  );
}

export async function mintTo(): Promise<void> {
  await testToken.mintTo(testAccount, testMintAuthority, [], 1000);

  const mintInfo = await testToken.getMintInfo();
  assert(mintInfo.supply.toNumber() === 1000);

  const accountInfo = await testToken.getAccountInfo(testAccount);
  assert(accountInfo.amount.toNumber() === 1000);
}

export async function mintToChecked(): Promise<void> {
  assert(
    await didThrow(async () => {
      await testToken.mintToChecked(
        testAccount,
        testMintAuthority,
        [],
        1000,
        1,
      );
    }),
  );

  await testToken.mintToChecked(testAccount, testMintAuthority, [], 1000, 2);

  const mintInfo = await testToken.getMintInfo();
  assert(mintInfo.supply.toNumber() === 2000);

  const accountInfo = await testToken.getAccountInfo(testAccount);
  assert(accountInfo.amount.toNumber() === 2000);
}

export async function transfer(): Promise<void> {
  const destOwner = Keypair.generate();
  const dest = await testToken.createAccount(destOwner.publicKey);

  await testToken.transfer(testAccount, dest, testAccountOwner, [], 100);

  const mintInfo = await testToken.getMintInfo();
  assert(mintInfo.supply.toNumber() === 2000);

  let destAccountInfo = await testToken.getAccountInfo(dest);
  assert(destAccountInfo.amount.toNumber() === 100);

  let testAccountInfo = await testToken.getAccountInfo(testAccount);
  assert(testAccountInfo.amount.toNumber() === 1900);
}

export async function transferChecked(): Promise<void> {
  const destOwner = Keypair.generate();
  const dest = await testToken.createAccount(destOwner.publicKey);

  assert(
    await didThrow(async () => {
      await testToken.transferChecked(
        testAccount,
        dest,
        testAccountOwner,
        [],
        100,
        testTokenDecimals - 1,
      );
    }),
  );

  await testToken.transferChecked(
    testAccount,
    dest,
    testAccountOwner,
    [],
    100,
    testTokenDecimals,
  );

  const mintInfo = await testToken.getMintInfo();
  assert(mintInfo.supply.toNumber() === 2000);

  let destAccountInfo = await testToken.getAccountInfo(dest);
  assert(destAccountInfo.amount.toNumber() === 100);

  let testAccountInfo = await testToken.getAccountInfo(testAccount);
  assert(testAccountInfo.amount.toNumber() === 1800);
}

export async function transferCheckedAssociated(): Promise<void> {
  const dest = Keypair.generate().publicKey;
  let associatedAccount;

  associatedAccount = await testToken.getOrCreateAssociatedAccountInfo(dest);
  assert(associatedAccount.amount.toNumber() === 0);

  await testToken.transferChecked(
    testAccount,
    associatedAccount.address,
    testAccountOwner,
    [],
    123,
    testTokenDecimals,
  );

  associatedAccount = await testToken.getOrCreateAssociatedAccountInfo(dest);
  assert(associatedAccount.amount.toNumber() === 123);
}

export async function approveRevoke(): Promise<void> {
  const delegate = Keypair.generate().publicKey;

  await testToken.approve(testAccount, delegate, testAccountOwner, [], 42);

  let testAccountInfo = await testToken.getAccountInfo(testAccount);
  assert(testAccountInfo.delegatedAmount.toNumber() === 42);
  if (testAccountInfo.delegate === null) {
    throw new Error('delegate should not be null');
  } else {
    assert(testAccountInfo.delegate.equals(delegate));
  }

  await testToken.revoke(testAccount, testAccountOwner, []);

  testAccountInfo = await testToken.getAccountInfo(testAccount);
  assert(testAccountInfo.delegatedAmount.toNumber() === 0);
  if (testAccountInfo.delegate !== null) {
    throw new Error('delegate should be null');
  }
}

export async function failOnApproveOverspend(): Promise<void> {
  const owner = Keypair.generate();
  const account1 = await testToken.createAccount(owner.publicKey);
  const account2 = await testToken.createAccount(owner.publicKey);
  const delegate = Keypair.generate();

  await testToken.transfer(testAccount, account1, testAccountOwner, [], 10);
  await testToken.approve(account1, delegate.publicKey, owner, [], 2);

  let account1Info = await testToken.getAccountInfo(account1);
  assert(account1Info.amount.toNumber() == 10);
  assert(account1Info.delegatedAmount.toNumber() == 2);
  if (account1Info.delegate === null) {
    throw new Error('delegate should not be null');
  } else {
    assert(account1Info.delegate.equals(delegate.publicKey));
  }

  await testToken.transfer(account1, account2, delegate, [], 1);

  account1Info = await testToken.getAccountInfo(account1);
  assert(account1Info.amount.toNumber() == 9);
  assert(account1Info.delegatedAmount.toNumber() == 1);

  await testToken.transfer(account1, account2, delegate, [], 1);

  account1Info = await testToken.getAccountInfo(account1);
  assert(account1Info.amount.toNumber() == 8);
  assert(account1Info.delegate === null);
  assert(account1Info.delegatedAmount.toNumber() == 0);

  assert(
    await didThrow(async () => {
      await testToken.transfer(account1, account2, delegate, [], 1);
    }),
  );
}

export async function setAuthority(): Promise<void> {
  const newOwner = Keypair.generate();
  await testToken.setAuthority(
    testAccount,
    newOwner.publicKey,
    'AccountOwner',
    testAccountOwner,
    [],
  );
  assert(
    await didThrow(async () => {
      await testToken.setAuthority(
        testAccount,
        newOwner.publicKey,
        'AccountOwner',
        testAccountOwner,
        [],
      );
    }),
  );
  await testToken.setAuthority(
    testAccount,
    testAccountOwner.publicKey,
    'AccountOwner',
    newOwner,
    [],
  );
}

export async function burn(): Promise<void> {
  let accountInfo = await testToken.getAccountInfo(testAccount);
  const amount = accountInfo.amount.toNumber();

  await testToken.burn(testAccount, testAccountOwner, [], 1);

  accountInfo = await testToken.getAccountInfo(testAccount);
  assert(accountInfo.amount.toNumber() == amount - 1);
}

export async function burnChecked(): Promise<void> {
  let accountInfo = await testToken.getAccountInfo(testAccount);
  const amount = accountInfo.amount.toNumber();

  assert(
    await didThrow(async () => {
      await testToken.burnChecked(testAccount, testAccountOwner, [], 1, 1);
    }),
  );

  await testToken.burnChecked(testAccount, testAccountOwner, [], 1, 2);

  accountInfo = await testToken.getAccountInfo(testAccount);
  assert(accountInfo.amount.toNumber() == amount - 1);
}

export async function freezeThawAccount(): Promise<void> {
  let accountInfo = await testToken.getAccountInfo(testAccount);
  const amount = accountInfo.amount.toNumber();

  await testToken.freezeAccount(testAccount, testMintAuthority, []);

  const destOwner = Keypair.generate();
  const dest = await testToken.createAccount(destOwner.publicKey);

  assert(
    await didThrow(async () => {
      await testToken.transfer(testAccount, dest, testAccountOwner, [], 100);
    }),
  );

  await testToken.thawAccount(testAccount, testMintAuthority, []);

  await testToken.transfer(testAccount, dest, testAccountOwner, [], 100);

  let testAccountInfo = await testToken.getAccountInfo(testAccount);
  assert(testAccountInfo.amount.toNumber() === amount - 100);
}

export async function closeAccount(): Promise<void> {
  const closeAuthority = Keypair.generate();

  await testToken.setAuthority(
    testAccount,
    closeAuthority.publicKey,
    'CloseAccount',
    testAccountOwner,
    [],
  );
  let accountInfo = await testToken.getAccountInfo(testAccount);
  if (accountInfo.closeAuthority === null) {
    assert(accountInfo.closeAuthority !== null);
  } else {
    assert(accountInfo.closeAuthority.equals(closeAuthority.publicKey));
  }

  const dest = await testToken.createAccount(Keypair.generate().publicKey);
  const remaining = accountInfo.amount.toNumber();

  // Check that accounts with non-zero token balance cannot be closed
  assert(
    await didThrow(async () => {
      await testToken.closeAccount(testAccount, dest, closeAuthority, []);
    }),
  );

  const connection = await getConnection();
  let tokenRentExemptAmount;
  let info = await connection.getAccountInfo(testAccount);
  if (info != null) {
    tokenRentExemptAmount = info.lamports;
  } else {
    throw new Error('Account not found');
  }

  // Transfer away all tokens
  await testToken.transfer(testAccount, dest, testAccountOwner, [], remaining);

  // Close for real
  await testToken.closeAccount(testAccount, dest, closeAuthority, []);

  info = await connection.getAccountInfo(testAccount);
  assert(info === null);

  let destInfo = await connection.getAccountInfo(dest);
  if (destInfo !== null) {
    assert(destInfo.lamports === 2 * tokenRentExemptAmount);
  } else {
    throw new Error('Account not found');
  }

  let destAccountInfo = await testToken.getAccountInfo(dest);
  assert(destAccountInfo.amount.toNumber() === remaining);
}

export async function multisig(): Promise<void> {
  const m = 2;
  const n = 5;

  let signerAccounts = [];
  for (var i = 0; i < n; i++) {
    signerAccounts.push(Keypair.generate());
  }
  let signerPublicKeys = [];
  signerAccounts.forEach(account => signerPublicKeys.push(account.publicKey));
  const multisig = await testToken.createMultisig(m, signerPublicKeys);
  const multisigInfo = await testToken.getMultisigInfo(multisig);
  assert(multisigInfo.m === m);
  assert(multisigInfo.n === n);
  assert(multisigInfo.signer1.equals(signerPublicKeys[0]));
  assert(multisigInfo.signer2.equals(signerPublicKeys[1]));
  assert(multisigInfo.signer3.equals(signerPublicKeys[2]));
  assert(multisigInfo.signer4.equals(signerPublicKeys[3]));
  assert(multisigInfo.signer5.equals(signerPublicKeys[4]));

  const multisigOwnedAccount = await testToken.createAccount(multisig);
  const finalDest = await testToken.createAccount(multisig);

  await testToken.mintTo(multisigOwnedAccount, testMintAuthority, [], 1000);

  // Transfer via multisig
  await testToken.transfer(
    multisigOwnedAccount,
    finalDest,
    multisig,
    signerAccounts,
    1,
  );
  await sleep(500);
  let accountInfo = await testToken.getAccountInfo(finalDest);
  assert(accountInfo.amount.toNumber() == 1);

  // Approve via multisig
  {
    const delegate = new PublicKey(0);
    await testToken.approve(
      multisigOwnedAccount,
      delegate,
      multisig,
      signerAccounts,
      1,
    );
    const accountInfo = await testToken.getAccountInfo(multisigOwnedAccount);
    assert(accountInfo.delegate != null);
    if (accountInfo.delegate != null) {
      assert(accountInfo.delegate.equals(delegate));
      assert(accountInfo.delegatedAmount.toNumber() == 1);
    }
  }

  // SetAuthority of account via multisig
  {
    const newOwner = new PublicKey(0);
    await testToken.setAuthority(
      multisigOwnedAccount,
      newOwner,
      'AccountOwner',
      multisig,
      signerAccounts,
    );
    const accountInfo = await testToken.getAccountInfo(multisigOwnedAccount);
    assert(accountInfo.owner.equals(newOwner));
  }
}

export async function nativeToken(): Promise<void> {
  const connection = await getConnection();
  // this user both pays for the creation of the new token account
  // and provides the lamports to wrap
  const payer = await newAccountWithLamports(connection, 2000000000 /* wag */);
  const lamportsToWrap = 1000000000;

  const token = new Token(connection, NATIVE_MINT, programId, payer);
  const owner = Keypair.generate();
  const native = await Token.createWrappedNativeAccount(
    connection,
    programId,
    owner.publicKey,
    payer,
    lamportsToWrap,
  );
  let accountInfo = await token.getAccountInfo(native);
  assert(accountInfo.isNative);

  // check that the new account has wrapped native tokens.
  assert(accountInfo.amount.toNumber() === lamportsToWrap);

  let balance;
  let info = await connection.getAccountInfo(native);
  if (info != null) {
    balance = info.lamports;
  } else {
    throw new Error('Account not found');
  }

  const programVersion = process.env.PROGRAM_VERSION;
  if (!programVersion) {
    // transfer lamports into the native account
    const additionalLamports = 100;
    await sendAndConfirmTransaction(
      'TransferLamports',
      connection,
      new Transaction().add(
        SystemProgram.transfer({
          fromPubkey: payer.publicKey,
          toPubkey: native,
          lamports: additionalLamports,
        }),
      ),
      payer,
    );

    // no change in the amount
    accountInfo = await token.getAccountInfo(native);
    assert(accountInfo.amount.toNumber() === lamportsToWrap);

    // sync, amount changes
    await token.syncNative(native);
    accountInfo = await token.getAccountInfo(native);
    assert(
      accountInfo.amount.toNumber() === lamportsToWrap + additionalLamports,
    );
    balance += additionalLamports;
  }

  const balanceNeeded = await connection.getMinimumBalanceForRentExemption(0);
  const dest = await newAccountWithLamports(connection, balanceNeeded);
  await token.closeAccount(native, dest.publicKey, owner, []);
  info = await connection.getAccountInfo(native);
  if (info != null) {
    throw new Error('Account not burned');
  }
  info = await connection.getAccountInfo(dest.publicKey);
  if (info != null) {
    assert(info.lamports == balanceNeeded + balance);
  } else {
    throw new Error('Account not found');
  }
}
