import { fs } from 'mz';
import {
  Account,
  Connection,
  BpfLoader,
  PublicKey,
  BPF_LOADER_PROGRAM_ID,
} from '@solana/web3.js';

import { url } from '../src/client/util/url';
import { newAccountWithLamports } from '../src/client/util/new-account-with-lamports';
import { Store } from '../src/client/util/store';
import { Identity } from '../src/client/identity';

const IDENTITY_PROGRAM_BUILD_PATH =
  '../../target/bpfel-unknown-unknown/release/spl_identity.so';

// Loaded identity program's program id
let programId: PublicKey;

let connection: Connection;
async function getConnection(): Promise<Connection> {
  if (connection) return connection;

  connection = new Connection(url, 'recent');
  const version = await connection.getVersion();

  console.log('Connection to cluster established:', url, version);
  return connection;
}

async function loadProgram(
  connection: Connection,
  path: string
): Promise<PublicKey> {
  const NUM_RETRIES = 500; /* allow some number of retries */
  const data = await fs.readFile(path);
  const { feeCalculator } = await connection.getRecentBlockhash();
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
    BPF_LOADER_PROGRAM_ID
  );
  return program_account.publicKey;
}

async function GetPrograms(connection: Connection): Promise<PublicKey> {
  const store = new Store();
  try {
    const config = await store.load('config.json');
    console.log('Using pre-loaded Identity program');
    console.log(
      '  Note: To reload program remove client/util/store/config.json'
    );
    return new PublicKey(config.identityProgramId);
  } catch (err) {
    const identityProgramId = await loadProgram(
      connection,
      IDENTITY_PROGRAM_BUILD_PATH
    );
    await store.save('config.json', {
      identityProgramId: identityProgramId.toString(),
    });

    return identityProgramId;
  }
}

export async function loadIdentityProgram(payer: Account): Promise<Identity> {
  const connection = await getConnection();
  programId = await GetPrograms(connection);

  console.log('Identity Program ID', programId.toString());

  return new Identity(connection, programId, payer);
}

export async function createAccount(
  airdropLamports: number = 1000000000
): Promise<Account> {
  const connection = await getConnection();
  return newAccountWithLamports(connection, airdropLamports);
}
