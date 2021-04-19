/* eslint-disable @typescript-eslint/no-unsafe-assignment */
/* eslint-disable @typescript-eslint/no-unsafe-member-access */
/* eslint-disable @typescript-eslint/no-unsafe-call */
/* eslint-disable @typescript-eslint/ban-ts-comment */

import {
  Account,
  Connection,
  BpfLoader,
  BPF_LOADER_PROGRAM_ID,
  PublicKey,
  LAMPORTS_PER_SOL,
  SystemProgram,
  TransactionInstruction,
  Transaction,
  sendAndConfirmTransaction,
} from '@solana/web3.js';
import fs from 'mz/fs';

// @ts-ignore
import BufferLayout from 'buffer-layout';

import {url, urlTls} from './util/url';
import {Store} from './util/store';
import {newAccountWithLamports} from './util/new-account-with-lamports';

import * as Layout from './util/layout';


/**
 * Connection to the network
 */
let connection: Connection;

/**
 * Connection to the network
 */
let payerAccount: Account;

/**
 * Hello world's program id
 */
let programId: PublicKey;

let securityPubkey: PublicKey;

/**
 * The public key of the account we are saying hello to
 */
let greetedPubkey: PublicKey;

const pathToProgram = 'dist/program/helloworld.so';

/**
 * Layout of the greeted account data
 */
const greetedAccountDataLayout = BufferLayout.struct([
  BufferLayout.u32('numGreets'),
]);

/**
 * Layout of the program security account data
 */
const securityAccountDataLayout = BufferLayout.struct([
  BufferLayout.u32('ownerOption'),
  Layout.publicKey('owner'),
  BufferLayout.u8('paused'),
]);

/**
 * Establish a connection to the cluster
 */
export async function establishConnection(): Promise<void> {
  connection = new Connection(url, 'singleGossip');
  const version = await connection.getVersion();
  console.log('Connection to cluster established:', url, version);
}

/**
 * Establish an account to pay for everything
 */
export async function establishPayer(): Promise<void> {
  try {
    const config = await new Store().load('payer.json');
    payerAccount = new Account(Uint8Array.from(config.secretKey.split(',').map(Number)))
    console.log('loaded previous payer from payer.json')
  } catch (e) {
    // pass
  }
  if (!payerAccount) {
    let fees = 0;
    const {feeCalculator} = await connection.getRecentBlockhash();

    // Calculate the cost to load the program
    const data = await fs.readFile(pathToProgram);
    const NUM_RETRIES = 500; // allow some number of retries
    fees +=
      feeCalculator.lamportsPerSignature *
        (BpfLoader.getMinNumSignatures(data.length) + NUM_RETRIES) +
      (await connection.getMinimumBalanceForRentExemption(data.length));

    // Calculate the cost to fund the greeter account
    fees += await connection.getMinimumBalanceForRentExemption(
      greetedAccountDataLayout.span,
    );

    // Calculate the cost of sending the transactions
    fees += feeCalculator.lamportsPerSignature * 100; // wag

    // Fund a new payer via airdrop
    payerAccount = await newAccountWithLamports(connection, fees);

    await new Store().save('payer.json', { secretKey: payerAccount.secretKey.toString() });
  }

  const lamports = await connection.getBalance(payerAccount.publicKey);
  console.log(
    'Using account',
    payerAccount.publicKey.toBase58(),
    'containing',
    lamports / LAMPORTS_PER_SOL,
    'Sol to pay for fees',
  );
}

/**
 * Load the hello world BPF program if not already loaded
 */
export async function loadProgram(): Promise<void> {
  const store = new Store();

  // Check if the program has already been loaded
  try {
    const config = await store.load('config.json');
    programId = new PublicKey(config.programId);
    securityPubkey = new PublicKey(config.securityPubkey);
    greetedPubkey = new PublicKey(config.greetedPubkey);
    await connection.getAccountInfo(programId);
    console.log('Program already loaded to account', programId.toBase58());
    console.log('security', securityPubkey.toBase58());
    console.log('greeted ', greetedPubkey.toBase58());
    return;
  } catch (err) {
    // try to load the program
  }

  // Load the program
  console.log('Loading hello world program...');
  const data = await fs.readFile(pathToProgram);
  const programAccount = new Account();
  await BpfLoader.load(
    connection,
    payerAccount,
    programAccount,
    data,
    BPF_LOADER_PROGRAM_ID,
  );
  programId = programAccount.publicKey;
  console.log('Program loaded to account', programId.toBase58());

  async function createAccount(dataLayout: BufferLayout): Promise<PublicKey> {
      // Create the new account
      const newAccount = new Account();
      const pubkey = newAccount.publicKey;
      console.log('Creating account', pubkey.toBase58());
      const space = dataLayout.span;
      const lamports = await connection.getMinimumBalanceForRentExemption(space);
      const transaction = new Transaction().add(
        SystemProgram.createAccount({
          fromPubkey: payerAccount.publicKey,
          newAccountPubkey: pubkey,
          lamports,
          space,
          programId,
        }),
      );
      await sendAndConfirmTransaction(
        connection,
        transaction,
        [payerAccount, newAccount],
        {
          commitment: 'singleGossip',
          preflightCommitment: 'singleGossip',
        },
      );
      return newAccount.publicKey;
  }
  securityPubkey = await createAccount(securityAccountDataLayout);
  greetedPubkey = await createAccount(greetedAccountDataLayout);

  // Save this info for next time
  await store.save('config.json', {
    url: urlTls,
    programId: programId.toBase58(),
    securityPubkey: securityPubkey.toBase58(),
    greetedPubkey: greetedPubkey.toBase58(),
  });

  reportOwner()
}

/**
 * Say hello
 */
export async function sayHello(): Promise<void> {
  console.log('Saying hello to', greetedPubkey.toBase58(), 'security', securityPubkey.toBase58());
  const instruction = new TransactionInstruction({
    keys: [
      {pubkey: securityPubkey, isSigner: false, isWritable: true},
      {pubkey: greetedPubkey, isSigner: false, isWritable: true},
    ],
    programId,
    data: Buffer.alloc(0), // All instructions are hellos
  });
  await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [payerAccount],
    {
      commitment: 'singleGossip',
      preflightCommitment: 'singleGossip',
    },
  );
  console.log('sayHello done')
}

/**
 * Report the number of times the greeted account has been said hello to
 */
export async function reportHellos(): Promise<void> {
  console.log('Report Hellos to', greetedPubkey.toBase58());
  const accountInfo = await connection.getAccountInfo(greetedPubkey);
  if (accountInfo === null) {
    throw 'Error: cannot find the greeted account';
  }
  const info = greetedAccountDataLayout.decode(Buffer.from(accountInfo.data));
  console.log(
    greetedPubkey.toBase58(),
    'has been greeted',
    info.numGreets.toString(),
    'times',
  );
}

export async function reportOwner(): Promise<void> {
  const accountInfo = await connection.getAccountInfo(securityPubkey);
  if (accountInfo === null) {
    throw 'Error: cannot find the program account';
  }
  const info = securityAccountDataLayout.decode(Buffer.from(accountInfo.data));
  console.log(
    'CLIENT',
    securityPubkey.toBase58(),
    info.ownerOption === 0 ? 'has NO owner' : 
    ('has owner ' + new PublicKey(info.owner,).toBase58()),
  );
}


export async function initializeOwnership(): Promise<void> {
  console.log('Initializing Owner to Payer', payerAccount.publicKey.toBase58());
  const accountInfo = await connection.getAccountInfo(securityPubkey);
  if (accountInfo) {
    const info = securityAccountDataLayout.decode(Buffer.from(accountInfo.data));
    if (info.ownerOption !== 0) {
      console.log('... already owned by', new PublicKey(info.owner,).toBase58());
      return
    }
  }

  const dataLayout = BufferLayout.struct([
    BufferLayout.u8('instruction'),
    Layout.publicKey('owner'),
  ])
  const data = Buffer.alloc(dataLayout.span);
  dataLayout.encode({instruction: 0 /* InitializeOwnership */}, data);

  const instruction = new TransactionInstruction({
    keys: [
      {pubkey: securityPubkey, isSigner: false, isWritable: true},
      {pubkey: payerAccount.publicKey, isSigner: false, isWritable: false},
    ],
    programId,
    data,
  });
  await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [payerAccount],
    {
      commitment: 'singleGossip',
      preflightCommitment: 'singleGossip',
    },
  );
}
  
export async function transferOwnershipToGreeted(): Promise<void> {
  console.log('Transfer ownership to Greeted', greetedPubkey.toBase58());
  return _transferOwnership(payerAccount.publicKey, greetedPubkey)
}

export async function transferOwnershipToPayer(): Promise<void> {
  console.log('Transfer ownership to Payer', payerAccount.publicKey.toBase58());
  return _transferOwnership(greetedPubkey, payerAccount.publicKey)
}

async function _transferOwnership(currentOwnerPubkey: PublicKey, newOwnerPubkey: PublicKey) {
  const dataLayout = BufferLayout.struct([
    BufferLayout.u8('instruction'),
    Layout.publicKey('owner'),
  ])
  const data = Buffer.alloc(dataLayout.span);
  dataLayout.encode({
    instruction: 1 /* TransferOwnership */,
    //owner: Buffer.from(newOwnerPubkey.toBuffer()), // TODO get this working
  }, data);

  const instruction = new TransactionInstruction({
    keys: [
      {pubkey: securityPubkey, isSigner: false, isWritable: true},
      {pubkey: currentOwnerPubkey, isSigner: false, isWritable: false},
      {pubkey: newOwnerPubkey, isSigner: false, isWritable: false},
    ],
    programId,
    data,
  });
  await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [payerAccount],
    {
      commitment: 'singleGossip',
      preflightCommitment: 'singleGossip',
    },
  );
}
 
export async function reportPaused(): Promise<void> {
  const accountInfo = await connection.getAccountInfo(securityPubkey);
  if (accountInfo === null) {
    throw 'Error: cannot find the program account';
  }
  const info = securityAccountDataLayout.decode(Buffer.from(accountInfo.data));
  console.log(
    'CLIENT',
    securityPubkey.toBase58(),
    info.paused ? 'is paused' : 'is not paused',
  );
}

export async function pauseByPayer(): Promise<void> {
    console.log('Pausing by Payer');
    return _pause(payerAccount.publicKey)
}

export async function pauseByGreeted(): Promise<void> {
    console.log('Pausing by Greeter');
    return _pause(greetedPubkey)
}

export async function resumeByPayer(): Promise<void> {
    console.log('Resuming by Payer');
    return _resume(payerAccount.publicKey)
}

export async function resumeByGreeted(): Promise<void> {
    console.log('Resuming by Greeter');
    return _resume(greetedPubkey)
}

async function _pause(pausePubkey: PublicKey): Promise<void> {
  const accountInfo = await connection.getAccountInfo(securityPubkey);

  const dataLayout = BufferLayout.struct([BufferLayout.u8('instruction')])
  const data = Buffer.alloc(dataLayout.span);
  dataLayout.encode({instruction: 3 /* Pause */}, data);

  const instruction = new TransactionInstruction({
    keys: [
      {pubkey: securityPubkey, isSigner: false, isWritable: true},
      {pubkey: pausePubkey, isSigner: false, isWritable: false},
    ],
    programId,
    data,
  });
  await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [payerAccount],
    {
      commitment: 'singleGossip',
      preflightCommitment: 'singleGossip',
    },
  );
}

async function _resume(resumePubkey: PublicKey): Promise<void> {
  const accountInfo = await connection.getAccountInfo(securityPubkey);

  const dataLayout = BufferLayout.struct([BufferLayout.u8('instruction')])
  const data = Buffer.alloc(dataLayout.span);
  dataLayout.encode({instruction: 4 /* Resume */}, data);

  const instruction = new TransactionInstruction({
    keys: [
      {pubkey: securityPubkey, isSigner: false, isWritable: true},
      {pubkey: resumePubkey, isSigner: false, isWritable: false},
    ],
    programId,
    data,
  });
  await sendAndConfirmTransaction(
    connection,
    new Transaction().add(instruction),
    [payerAccount],
    {
      commitment: 'singleGossip',
      preflightCommitment: 'singleGossip',
    },
  );
}
