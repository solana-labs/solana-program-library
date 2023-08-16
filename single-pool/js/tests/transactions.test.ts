import test from 'ava';
import { start, BanksClient, ProgramTestContext } from 'solana-bankrun';
import {
  Keypair,
  VoteAccount,
  LAMPORTS_PER_SOL,
  Connection,
  PublicKey,
  Transaction,
  SystemProgram,
  Authorized,
  StakeProgram,
  VoteProgram,
} from '@solana/web3.js';
import {
  SINGLE_POOL_PROGRAM_ID,
  MPL_METADATA_PROGRAM_ID,
  findPoolAddress,
  findPoolStakeAddress,
  findPoolMintAddress,
  initialize,
  deposit,
  withdraw,
  createTokenMetadata,
  findMplMetadataAddress,
} from '../src/index.ts';
import * as voteAccount from './vote_account.json';

class BanksConnection {
  constructor(client: BanksClient) {
    this.client = client;
  }

  async getMinimumBalanceForRentExemption(dataLen: number): Promise<number> {
    const rent = await this.client.getRent();
    return Number(rent.minimumBalance(BigInt(dataLen)));
  }

  async getStakeMinimumDelegation() {
    // TODO add this rpc call to the banks client
    return { value: LAMPORTS_PER_SOL };
  }

  async getAccountInfo(address: PublicKey, commitment?: string): Promise<AccountInfo<Buffer>> {
    const account = await this.client.getAccount(address, commitment);
    return account ? account.toBuffer() : account;
  }
}

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

async function processTransaction(
  context: ProgramTestContext,
  transaction: Transaction,
  signers = [],
) {
  transaction.recentBlockhash = context.lastBlockhash;
  transaction.feePayer = context.payer.publicKey;
  transaction.sign(...[context.payer].concat(signers));
  return context.banksClient.processTransaction(transaction);
}

async function createAndDelegateStakeAccount(
  context: ProgramTestContext,
  voteAccountAddress: PublicKey,
): Promise<PublicKey> {
  const connection = new BanksConnection(context.banksClient);
  let userStakeAccount = new Keypair();

  const stakeRent = await connection.getMinimumBalanceForRentExemption(StakeProgram.space);
  const minimumDelegation = (await connection.getStakeMinimumDelegation()).value;
  let transaction = StakeProgram.createAccount({
    authorized: new Authorized(context.payer.publicKey, context.payer.publicKey),
    fromPubkey: context.payer.publicKey,
    lamports: stakeRent + minimumDelegation,
    stakePubkey: userStakeAccount.publicKey,
  });
  await processTransaction(context, transaction, [userStakeAccount]);
  userStakeAccount = userStakeAccount.publicKey;

  transaction = StakeProgram.delegate({
    authorizedPubkey: context.payer.publicKey,
    stakePubkey: userStakeAccount,
    votePubkey: voteAccountAddress,
  });
  await processTransaction(context, transaction);

  return userStakeAccount;
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
  await processTransaction(context, transaction);

  t.truthy(await client.getAccount(poolAddress), 'pool has been created');
  t.truthy(
    await client.getAccount(
      findMplMetadataAddress(findPoolMintAddress(SINGLE_POOL_PROGRAM_ID, poolAddress)),
    ),
    'metadata has been created',
  );
});

test('deposit', async (t) => {
  const context = await startWithContext();
  const client = context.banksClient;
  const connection = new BanksConnection(client);
  const payer = context.payer;

  const voteAccountAddress = new PublicKey(voteAccount.pubkey);
  const poolAddress = findPoolAddress(SINGLE_POOL_PROGRAM_ID, voteAccountAddress);
  const poolStakeAddress = findPoolStakeAddress(SINGLE_POOL_PROGRAM_ID, poolAddress);
  const userStakeAccount = await createAndDelegateStakeAccount(context, voteAccountAddress);

  // initialize pool
  let transaction = await initialize(connection, voteAccountAddress, payer.publicKey);
  await processTransaction(context, transaction);

  // deposit
  transaction = await deposit({
    connection,
    pool: poolAddress,
    userWallet: payer.publicKey,
    userStakeAccount,
  });
  await processTransaction(context, transaction);

  const minimumDelegation = (await connection.getStakeMinimumDelegation()).value;
  const poolStakeAccount = await client.getAccount(poolStakeAddress);
  t.true(poolStakeAccount.lamports > minimumDelegation * 2, 'stake has been deposited');
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
  await processTransaction(context, transaction);

  t.truthy(await client.getAccount(poolAddress), 'pool has been created');
  t.falsy(
    await client.getAccount(
      findMplMetadataAddress(findPoolMintAddress(SINGLE_POOL_PROGRAM_ID, poolAddress)),
    ),
    'metadata has not been created',
  );

  // create metadata
  transaction = await createTokenMetadata(poolAddress, payer.publicKey);
  await processTransaction(context, transaction);

  t.truthy(
    await client.getAccount(
      findMplMetadataAddress(findPoolMintAddress(SINGLE_POOL_PROGRAM_ID, poolAddress)),
    ),
    'metadata has been created',
  );
});
