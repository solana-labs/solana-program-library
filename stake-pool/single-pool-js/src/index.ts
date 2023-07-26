import {
  PublicKey,
  Connection,
  TransactionInstruction,
  LAMPORTS_PER_SOL,
  SYSVAR_RENT_PUBKEY,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_STAKE_HISTORY_PUBKEY,
  Transaction,
  STAKE_CONFIG_ID,
  StakeProgram,
  sendAndConfirmTransaction,
  SystemProgram,
  Keypair,
  StakeAuthorizationLayout,
} from '@solana/web3.js';
import {
  TOKEN_PROGRAM_ID,
  MINT_SIZE,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
  createApproveInstruction,
} from '@solana/spl-token';
import * as BufferLayout from '@solana/buffer-layout';
import { Buffer } from 'buffer';
import fs from 'fs';

// solana-test-validator --reset --bpf-program 3cqnsMsT6LE96pxv7GR4di5rLqHDZZbR3FbeSUeRLFqY ~/work/solana/spl/target/deploy/spl_single_validator_pool.so --bpf-program metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s ~/work/solana/spl/stake-pool/program/tests/fixtures/mpl_token_metadata.so --account KRAKEnMdmT4EfM8ykTFH6yLoCd5vNLcQvJwF66Y2dag ~/vote_account.json

// XXX ok i think im giving up on web3 experimental for now its too complicated trying to work with it
// this is my fault and the fault of the npm corporation not the fault of the packages ofc
//
// ok so i need...
// * functions to get pda addresses
// * builders for each instruction
// * builders for transactions for the major functionality
// * types corresponding to information we need to represent eg the pool account
// * getters for useful info... pool stake/token supply, user stake/token balance...
//   getter for all single pools. think about what a dashboard would need
//
// split this shit into its own files later... just code it up

export const SINGLE_POOL_PROGRAM_ID = new PublicKey('3cqnsMsT6LE96pxv7GR4di5rLqHDZZbR3FbeSUeRLFqY');

// XXX pda fns

export function findPoolAddress(programId: PublicKey, voteAccountAddress: PublicKey) {
  return findPda(programId, voteAccountAddress, 'pool');
}

export function findPoolStakeAddress(programId: PublicKey, poolAddress: PublicKey) {
  return findPda(programId, poolAddress, 'stake');
}

export function findPoolMintAddress(programId: PublicKey, poolAddress: PublicKey) {
  return findPda(programId, poolAddress, 'mint');
}

export function findPoolStakeAuthorityAddress(programId: PublicKey, poolAddress: PublicKey) {
  return findPda(programId, poolAddress, 'stake_authority');
}

export function findPoolMintAuthorityAddress(programId: PublicKey, poolAddress: PublicKey) {
  return findPda(programId, poolAddress, 'mint_authority');
}

export function findPoolMplAuthorityAddress(programId: PublicKey, poolAddress: PublicKey) {
  return findPda(programId, poolAddress, 'mpl_authority');
}

function findPda(programId: PublicKey, baseAddress: PublicKey, prefix: string) {
  const [publicKey] = PublicKey.findProgramAddressSync(
    [Buffer.from(prefix), baseAddress.toBuffer()],
    programId,
  );
  return publicKey;
}

// TODO default deposit

// XXX instruction builders

export type InstructionType = {
  /** The Instruction index (from solana upstream program) */
  index: number;
  /** The BufferLayout to use to build data */
  layout: BufferLayout.Layout<any>;
};

export function encodeData(type: InstructionType, fields?: any): Buffer {
  const allocLength = type.layout.span;
  const data = Buffer.alloc(allocLength);
  const layoutFields = Object.assign({ instruction: type.index }, fields);
  type.layout.encode(layoutFields, data);

  return data;
}

export function decodeData(type: InstructionType, buffer: Buffer): any {
  let data;
  try {
    data = type.layout.decode(buffer);
  } catch (err) {
    throw new Error('invalid instruction; ' + err);
  }

  if (data.instruction !== type.index) {
    throw new Error(
      `invalid instruction; instruction index mismatch ${data.instruction} != ${type.index}`,
    );
  }

  return data;
}

export type SinglePoolInstructionType =
  | 'InitializePool'
  | 'DepositStake'
  | 'WithdrawStake'
  | 'CreateTokenMetadata'
  | 'UpdateTokenMetadata';

export const SINGLE_POOL_INSTRUCTION_LAYOUTS: {
  [type in SinglePoolInstructionType]: InstructionType;
} = Object.freeze({
  InitializePool: {
    index: 0,
    layout: BufferLayout.struct<any>([BufferLayout.u8('instruction')]),
  },
  DepositStake: {
    index: 1,
    layout: BufferLayout.struct<any>([BufferLayout.u8('instruction')]),
  },
  WithdrawStake: {
    index: 2,
    layout: BufferLayout.struct<any>([
      BufferLayout.u8('instruction'),
      BufferLayout.seq(BufferLayout.u8(), 32, 'userStakeAuthority'),
      BufferLayout.ns64('tokenAmount'),
    ]),
  },
  CreateTokenMetadata: {
    index: 3,
    layout: BufferLayout.struct<any>([BufferLayout.u8('instruction')]),
  },
  UpdateTokenMetadata: {
    index: 4,
    layout: BufferLayout.struct<any>([
      BufferLayout.u8('instruction'),
      BufferLayout.cstr('tokenName'),
      BufferLayout.cstr('tokenSymbol'),
      BufferLayout.cstr('tokenUri'),
    ]),
  },
});

// FIXME why does the stake pool js want program id for the pda search fns
// but hardcodes one for the instruction fns? seems odd
// FIXME should i use params objects for these?
export class SinglePoolInstruction {
  static initializePool(voteAccount: PublicKey): TransactionInstruction {
    const programId = SINGLE_POOL_PROGRAM_ID;
    const pool = findPoolAddress(programId, voteAccount);

    const keys = [
      { pubkey: voteAccount, isSigner: false, isWritable: false },
      { pubkey: pool, isSigner: false, isWritable: true },
      { pubkey: findPoolStakeAddress(programId, pool), isSigner: false, isWritable: true },
      { pubkey: findPoolMintAddress(programId, pool), isSigner: false, isWritable: true },
      {
        pubkey: findPoolStakeAuthorityAddress(programId, pool),
        isSigner: false,
        isWritable: false,
      },
      { pubkey: findPoolMintAuthorityAddress(programId, pool), isSigner: false, isWritable: false },
      { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_STAKE_HISTORY_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: STAKE_CONFIG_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: StakeProgram.programId, isSigner: false, isWritable: false },
    ];

    const type = SINGLE_POOL_INSTRUCTION_LAYOUTS.InitializePool;
    const data = encodeData(type);

    return new TransactionInstruction({
      programId,
      keys,
      data,
    });
  }

  static depositStake(
    pool: PublicKey,
    userStakeAccount: PublicKey,
    userTokenAccount: PublicKey,
    userLamportAccount: PublicKey,
  ): TransactionInstruction {
    const programId = SINGLE_POOL_PROGRAM_ID;

    const keys = [
      { pubkey: pool, isSigner: false, isWritable: false },
      { pubkey: findPoolStakeAddress(programId, pool), isSigner: false, isWritable: true },
      { pubkey: findPoolMintAddress(programId, pool), isSigner: false, isWritable: true },
      {
        pubkey: findPoolStakeAuthorityAddress(programId, pool),
        isSigner: false,
        isWritable: false,
      },
      { pubkey: findPoolMintAuthorityAddress(programId, pool), isSigner: false, isWritable: false },
      { pubkey: userStakeAccount, isSigner: false, isWritable: true },
      { pubkey: userTokenAccount, isSigner: false, isWritable: true },
      { pubkey: userLamportAccount, isSigner: false, isWritable: true },
      { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_STAKE_HISTORY_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: StakeProgram.programId, isSigner: false, isWritable: false },
    ];

    const type = SINGLE_POOL_INSTRUCTION_LAYOUTS.DepositStake;
    const data = encodeData(type);

    return new TransactionInstruction({
      programId,
      keys,
      data,
    });
  }

  static withdrawStake(
    pool: PublicKey,
    userStakeAccount: PublicKey,
    userStakeAuthority: PublicKey,
    userTokenAccount: PublicKey,
    userTokenAuthority: PublicKey,
    tokenAmount: number | bigint,
  ): TransactionInstruction {
    const programId = SINGLE_POOL_PROGRAM_ID;

    const keys = [
      { pubkey: pool, isSigner: false, isWritable: false },
      { pubkey: findPoolStakeAddress(programId, pool), isSigner: false, isWritable: true },
      { pubkey: findPoolMintAddress(programId, pool), isSigner: false, isWritable: true },
      {
        pubkey: findPoolStakeAuthorityAddress(programId, pool),
        isSigner: false,
        isWritable: false,
      },
      { pubkey: findPoolMintAuthorityAddress(programId, pool), isSigner: false, isWritable: false },
      { pubkey: userStakeAccount, isSigner: false, isWritable: true },
      { pubkey: userTokenAccount, isSigner: false, isWritable: true },
      { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: StakeProgram.programId, isSigner: false, isWritable: false },
    ];

    const type = SINGLE_POOL_INSTRUCTION_LAYOUTS.WithdrawStake;
    const data = encodeData(type, {
      userStakeAuthority: userStakeAuthority.toBuffer(),
      tokenAmount,
    });

    return new TransactionInstruction({
      programId,
      keys,
      data,
    });
  }
}

// XXX transaction builders
// FIXME should i use params objects for these too??

export async function initialize(connection: Connection, voteAccount: PublicKey, payer: PublicKey) {
  const transaction = new Transaction();

  const programId = SINGLE_POOL_PROGRAM_ID;
  const pool = findPoolAddress(programId, voteAccount);
  const stake = findPoolStakeAddress(programId, pool);
  const mint = findPoolMintAddress(programId, pool);

  const poolRent = await connection.getMinimumBalanceForRentExemption(33); // FIXME get buffer size in js
  const stakeRent = await connection.getMinimumBalanceForRentExemption(StakeProgram.space);
  const mintRent = await connection.getMinimumBalanceForRentExemption(MINT_SIZE);
  const minimumDelegation = (await connection.getStakeMinimumDelegation()).value;

  transaction.add(
    SystemProgram.transfer({
      fromPubkey: payer,
      toPubkey: pool,
      lamports: poolRent,
    }),
  );

  transaction.add(
    SystemProgram.transfer({
      fromPubkey: payer,
      toPubkey: stake,
      lamports: stakeRent + minimumDelegation,
    }),
  );

  transaction.add(
    SystemProgram.transfer({
      fromPubkey: payer,
      toPubkey: mint,
      lamports: mintRent,
    }),
  );

  transaction.add(SinglePoolInstruction.initializePool(voteAccount));

  return transaction;
}

export async function deposit(
  connection: Connection,
  pool: PublicKey,
  userWallet: PublicKey,
  userStakeAccount: PublicKey,
  userTokenAccount?: PublicKey,
  userLamportAccount?: PublicKey,
  userWithdrawAuthority?: PublicKey,
) {
  const transaction = new Transaction();

  const programId = SINGLE_POOL_PROGRAM_ID;
  const mint = findPoolMintAddress(programId, pool);
  const poolStakeAuthority = findPoolStakeAuthorityAddress(programId, pool);
  const userAssociatedTokenAccount = getAssociatedTokenAddressSync(mint, userWallet);

  if (!userTokenAccount) {
    userTokenAccount = userAssociatedTokenAccount;
  }

  if (!userLamportAccount) {
    userLamportAccount = userWallet;
  }

  if (!userWithdrawAuthority) {
    userWithdrawAuthority = userWallet;
  }

  if (
    userTokenAccount.equals(userAssociatedTokenAccount) &&
    (await connection.getAccountInfo(userAssociatedTokenAccount)) == null
  ) {
    transaction.add(
      createAssociatedTokenAccountInstruction(
        userWallet,
        userAssociatedTokenAccount,
        userWallet,
        mint,
      ),
    );
  }

  // TODO check token and stake account? (thats why i take connection and async)

  transaction.add(
    StakeProgram.authorize({
      stakePubkey: userStakeAccount,
      authorizedPubkey: userWithdrawAuthority,
      newAuthorizedPubkey: poolStakeAuthority,
      stakeAuthorizationType: StakeAuthorizationLayout.Staker,
    }),
  );

  transaction.add(
    StakeProgram.authorize({
      stakePubkey: userStakeAccount,
      authorizedPubkey: userWithdrawAuthority,
      newAuthorizedPubkey: poolStakeAuthority,
      stakeAuthorizationType: StakeAuthorizationLayout.Withdrawer,
    }),
  );

  transaction.add(
    SinglePoolInstruction.depositStake(
      pool,
      userStakeAccount,
      userTokenAccount,
      userLamportAccount,
    ),
  );

  return transaction;
}

// FIXME ok i need fucking params types ugh this is a fucking mess
export async function withdraw(
  connection: Connection,
  pool: PublicKey,
  userWallet: PublicKey,
  userStakeAccount: PublicKey,
  tokenAmount: number | bigint,
  createStakeAccount = false,
  userStakeAuthority?: PublicKey,
  userTokenAccount?: PublicKey,
  userTokenAuthority?: PublicKey,
) {
  const transaction = new Transaction();

  const programId = SINGLE_POOL_PROGRAM_ID;
  const poolMintAuthority = findPoolMintAuthorityAddress(programId, pool);

  if (createStakeAccount) {
    transaction.add(
      SystemProgram.createAccount({
        fromPubkey: userWallet,
        lamports: await connection.getMinimumBalanceForRentExemption(StakeProgram.space),
        newAccountPubkey: userStakeAccount,
        programId: StakeProgram.programId,
        space: StakeProgram.space,
      }),
    );
  }

  if (!userStakeAuthority) {
    userStakeAuthority = userWallet;
  }

  if (!userTokenAccount) {
    const mint = findPoolMintAddress(programId, pool);
    userTokenAccount = getAssociatedTokenAddressSync(mint, userWallet);
  }

  if (!userTokenAuthority) {
    userTokenAuthority = userWallet;
  }

  // TODO check token balance?

  transaction.add(
    createApproveInstruction(userTokenAccount, poolMintAuthority, userTokenAuthority, tokenAmount),
  );

  transaction.add(
    SinglePoolInstruction.withdrawStake(
      pool,
      userStakeAccount,
      userStakeAuthority,
      userTokenAccount,
      userTokenAuthority,
      tokenAmount,
    ),
  );

  return transaction;
}

async function main() {
  const connection = new Connection('http://127.0.0.1:8899', 'confirmed');
  const payer = Keypair.fromSecretKey(
    Uint8Array.from(JSON.parse(fs.readFileSync('/home/hana/.config/solana/id.json', 'utf8'))),
  );
  console.log('payer:', payer);
  console.log('keypair:', new Keypair());

  const voteAccount = new PublicKey('KRAKEnMdmT4EfM8ykTFH6yLoCd5vNLcQvJwF66Y2dag');
  const stakeAccount = new PublicKey('E1QPYQPWApgDpYiG4HRiiUauUYxS3iqxXGvzzz2RVj7u');
  const pool = findPoolAddress(SINGLE_POOL_PROGRAM_ID, voteAccount);

  let transaction = await initialize(connection, voteAccount, payer.publicKey);
  await sendAndConfirmTransaction(connection, transaction, [payer]);

  transaction = await deposit(connection, pool, payer.publicKey, stakeAccount);
  await sendAndConfirmTransaction(connection, transaction, [payer]);

  const userStakeAccount = new Keypair();
  transaction = await withdraw(
    connection,
    pool,
    payer.publicKey,
    userStakeAccount.publicKey,
    LAMPORTS_PER_SOL * 2,
    true,
  );
  await sendAndConfirmTransaction(connection, transaction, [payer, userStakeAccount]);
}

await main();
