import test from 'ava';
import { start, BanksClient } from 'solana-bankrun';
import {
  Keypair,
  VoteAccount,
  LAMPORTS_PER_SOL,
  Connection,
  PublicKey,
  Transaction,
  SystemProgram,
  VoteProgram,
} from '@solana/web3.js';
import {
  SINGLE_POOL_PROGRAM_ID,
  MPL_METADATA_PROGRAM_ID,
  findPoolAddress,
  findPoolMintAddress,
  initialize,
  createTokenMetadata,
  findMplMetadataAddress,
} from '../src/index.ts';
import * as voteAccount from './vote_account.json';

async function startWithContext(authorizedWithdrawer?: PublicKey) {
  const voteAccountData = Uint8Array.from(atob(voteAccount.account.data[0]), (c) =>
    c.charCodeAt(0),
  );

  if (authorizedWithdrawer != null) {
    voteAccountData.set(authorizedWithdrawer.toBytes(), 36);
  }

  return await start(
    [
      { name: 'spl_single_validator_pool', programId: SINGLE_POOL_PROGRAM_ID },
      { name: 'mpl_token_metadata', programId: MPL_METADATA_PROGRAM_ID },
    ],
    [
      {
        address: new PublicKey(voteAccount.pubkey),
        info: {
          lamports: voteAccount.account.lamports,
          data: voteAccountData,
          owner: VoteProgram.programId,
          executable: false,
        },
      },
    ],
  );
}

class BanksConnection {
  constructor(client: BanksClient) {
    this.client = client;
  }

  async getMinimumBalanceForRentExemption(dataLen: number): Promise<number> {
    const rent = await this.client.getRent();
    return Number(rent.minimumBalance(BigInt(dataLen)));
  }

  async getStakeMinimumDelegation(): Promise<number> {
    // TODO add this rpc call to the banks client
    return { value: LAMPORTS_PER_SOL };
  }
}

test('initialize', async (t) => {
  const context = await startWithContext();
  const client = context.banksClient;
  const connection = new BanksConnection(client);
  const payer = context.payer;

  const voteAccountAddress = new PublicKey(voteAccount.pubkey);
  const poolAddress = findPoolAddress(SINGLE_POOL_PROGRAM_ID, voteAccountAddress);

  // initialize pool
  const transaction = await initialize(connection, voteAccountAddress, payer.publicKey);
  const blockhash = context.lastBlockhash;
  transaction.recentBlockhash = blockhash;
  transaction.feePayer = payer.publicKey;
  transaction.sign(payer);
  await client.processTransaction(transaction);

  t.truthy(await client.getAccount(poolAddress), 'pool has been created');
  t.truthy(
    await client.getAccount(
      findMplMetadataAddress(findPoolMintAddress(SINGLE_POOL_PROGRAM_ID, poolAddress)),
    ),
    'metadata has been created',
  );
});

test('create metadata', async (t) => {
  const context = await startWithContext();
  const client = context.banksClient;
  const connection = new BanksConnection(client);
  const payer = context.payer;

  const voteAccountAddress = new PublicKey(voteAccount.pubkey);
  const poolAddress = findPoolAddress(SINGLE_POOL_PROGRAM_ID, voteAccountAddress);

  // initialize pool without metadata
  let transaction = await initialize(connection, voteAccountAddress, payer.publicKey, true);
  let blockhash = context.lastBlockhash;
  transaction.recentBlockhash = blockhash;
  transaction.feePayer = payer.publicKey;
  transaction.sign(payer);
  await client.processTransaction(transaction);

  t.truthy(await client.getAccount(poolAddress), 'pool has been created');
  t.falsy(
    await client.getAccount(
      findMplMetadataAddress(findPoolMintAddress(SINGLE_POOL_PROGRAM_ID, poolAddress)),
    ),
    'metadata has not been created',
  );

  // create metadata
  transaction = await createTokenMetadata(poolAddress, payer.publicKey);
  blockhash = context.lastBlockhash;
  transaction.recentBlockhash = blockhash;
  transaction.feePayer = payer.publicKey;
  transaction.sign(payer);
  await client.processTransaction(transaction);

  t.truthy(
    await client.getAccount(
      findMplMetadataAddress(findPoolMintAddress(SINGLE_POOL_PROGRAM_ID, poolAddress)),
    ),
    'metadata has been created',
  );
});
