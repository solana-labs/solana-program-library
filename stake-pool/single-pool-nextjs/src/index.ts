import {
  Base58EncodedAddress,
  setTransactionFeePayer,
  appendTransactionInstruction,
  signTransaction,
  getBase58EncodedAddressCodec,
  setTransactionLifetimeUsingBlockhash,
  createDefaultRpcTransport,
  createSolanaRpc,
  generateKeyPair,
  getBase64EncodedWireTransaction,
  getBase58EncodedAddressFromPublicKey,
  Transaction,
  TransactionVersion,
  AccountRole,
  IInstruction,
  IInstructionWithAccounts,
  IInstructionWithData,
  ReadonlySignerAccount,
  ReadonlyAccount,
  WritableAccount,
  getProgramDerivedAddress,
} from '@solana/web3.js';
import * as BufferLayout from '@solana/buffer-layout';
import { Buffer } from 'buffer';
import fs from 'fs';

//
//
// XXX bother luscher to add this stuff to the web3 library

function address<TAddress extends string>(string: TAddress): Base58EncodedAddress<TAddress> {
  return string as Base58EncodedAddress<TAddress>;
}

const SYSTEM_PROGRAM_ID = address('11111111111111111111111111111111');
const STAKE_PROGRAM_ID = address('Stake11111111111111111111111111111111111111');
const SYSVAR_RENT_ID = address('SysvarRent111111111111111111111111111111111');
const SYSVAR_CLOCK_ID = address('SysvarC1ock11111111111111111111111111111111');
const SYSVAR_STAKE_HISTORY_ID = address('SysvarStakeHistory1111111111111111111111111');
const STAKE_CONFIG_ID = address('StakeConfig11111111111111111111111111111111');

const STAKE_ACCOUNT_SIZE = BigInt(200);

// i could pr the system instructions but i have no idea what opinions luscher has about bufferlayout
class SystemInstruction {
  static transfer(params: {
    from: Base58EncodedAddress;
    to: Base58EncodedAddress;
    lamports: bigint;
  }): Instruction {
    const type = {
      index: 2,
      layout: BufferLayout.struct<{ instruction: number; lamports: bigint }>([
        BufferLayout.u32('instruction'),
        BufferLayout.nu64('lamports'),
      ]),
    };

    // XXX TODO FIXME wow i hate this i think bufferlayout doesnt support bigint at all?
    // it literally does this:
    // node_modules/@solana/buffer-layout/lib/Layout.js:626
    //    const hi32 = Math.floor(src / V2E32);
    // TypeError: Cannot mix BigInt and other types, use explicit conversions
    const data = encodeData(type, { lamports: Number(params.lamports) });

    const accounts = [
      { address: params.from, role: AccountRole.WRITABLE_SIGNER },
      { address: params.to, role: AccountRole.WRITABLE },
    ];

    return {
      data,
      accounts,
      programAddress: SYSTEM_PROGRAM_ID,
    };
  }
}

//
//
// XXX other non-us non-web3 nonsense

const TOKEN_PROGRAM_ID = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
const MINT_SIZE = BigInt(82);

//
//
// XXX our nonsense

export const SINGLE_POOL_PROGRAM_ID = address('3cqnsMsT6LE96pxv7GR4di5rLqHDZZbR3FbeSUeRLFqY');

// FIXME get pool buffer size via js layout span
const POOL_ACCOUNT_SIZE = BigInt(33);

//
//
// XXX account types, to prevent messing up order/inputs

export type VoteAccountAddress<TAddress extends Base58EncodedAddress = Base58EncodedAddress> =
  TAddress & {
    readonly __voteAccountAddress: unique symbol;
  };

export type PoolAddress<TAddress extends Base58EncodedAddress = Base58EncodedAddress> = TAddress & {
  readonly __poolAddress: unique symbol;
};

type PoolStakeAddress<TAddress extends Base58EncodedAddress = Base58EncodedAddress> = TAddress & {
  readonly __poolStakeAddress: unique symbol;
};

type PoolMintAddress<TAddress extends Base58EncodedAddress = Base58EncodedAddress> = TAddress & {
  readonly __poolMintAddress: unique symbol;
};

type PoolStakeAuthorityAddress<TAddress extends Base58EncodedAddress = Base58EncodedAddress> =
  TAddress & {
    readonly __poolStakeAuthorityAddress: unique symbol;
  };

type PoolMintAuthorityAddress<TAddress extends Base58EncodedAddress = Base58EncodedAddress> =
  TAddress & {
    readonly __poolMintAuthorityAddress: unique symbol;
  };

type PoolMplAuthorityAddress<TAddress extends Base58EncodedAddress = Base58EncodedAddress> =
  TAddress & {
    readonly __poolMplAuthorityAddress: unique symbol;
  };

//
//
// XXX pda fns

export async function findPoolAddress(
  programId: Base58EncodedAddress,
  voteAccountAddress: VoteAccountAddress,
): Promise<PoolAddress> {
  return (await findPda(programId, voteAccountAddress, 'pool')) as PoolAddress;
}

export async function findPoolStakeAddress(
  programId: Base58EncodedAddress,
  poolAddress: PoolAddress,
): Promise<PoolStakeAddress> {
  return (await findPda(programId, poolAddress, 'stake')) as PoolStakeAddress;
}

export async function findPoolMintAddress(
  programId: Base58EncodedAddress,
  poolAddress: PoolAddress,
): Promise<PoolMintAddress> {
  return (await findPda(programId, poolAddress, 'mint')) as PoolMintAddress;
}

export async function findPoolStakeAuthorityAddress(
  programId: Base58EncodedAddress,
  poolAddress: PoolAddress,
): Promise<PoolStakeAuthorityAddress> {
  return (await findPda(programId, poolAddress, 'stake_authority')) as PoolStakeAuthorityAddress;
}

export async function findPoolMintAuthorityAddress(
  programId: Base58EncodedAddress,
  poolAddress: PoolAddress,
): Promise<PoolMintAuthorityAddress> {
  return (await findPda(programId, poolAddress, 'mint_authority')) as PoolMintAuthorityAddress;
}

export async function findPoolMplAuthorityAddress(
  programId: Base58EncodedAddress,
  poolAddress: PoolAddress,
): Promise<PoolMplAuthorityAddress> {
  return (await findPda(programId, poolAddress, 'mpl_authority')) as PoolMplAuthorityAddress;
}

async function findPda(
  programId: Base58EncodedAddress,
  baseAddress: Base58EncodedAddress,
  prefix: string,
) {
  const { serialize } = getBase58EncodedAddressCodec();
  const { pda } = await getProgramDerivedAddress({
    programAddress: programId,
    seeds: [prefix, serialize(baseAddress)],
  });

  return pda;
}

//
//
// XXX ixn encode/decode

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

export type SinglePoolInstructionType = 'InitializePool';

export const SINGLE_POOL_INSTRUCTION_LAYOUTS: {
  [type in SinglePoolInstructionType]: InstructionType;
} = Object.freeze({
  InitializePool: {
    index: 0,
    layout: BufferLayout.struct<any>([BufferLayout.u8('instruction')]),
  },
});

//
//
// XXX ixn definitions

type InitializePoolInstruction = IInstruction<typeof SINGLE_POOL_PROGRAM_ID> &
  IInstructionWithAccounts<
    [
      ReadonlyAccount<VoteAccountAddress>,
      WritableAccount<PoolAddress>,
      WritableAccount<PoolStakeAddress>,
      WritableAccount<PoolMintAddress>,
      ReadonlyAccount<PoolStakeAuthorityAddress>,
      ReadonlyAccount<PoolMintAuthorityAddress>,
      ReadonlyAccount<typeof SYSVAR_RENT_ID>,
      ReadonlyAccount<typeof SYSVAR_CLOCK_ID>,
      ReadonlyAccount<typeof SYSVAR_STAKE_HISTORY_ID>,
      ReadonlyAccount<typeof STAKE_CONFIG_ID>,
      ReadonlyAccount<typeof SYSTEM_PROGRAM_ID>,
      ReadonlyAccount<typeof TOKEN_PROGRAM_ID>,
      ReadonlyAccount<typeof STAKE_PROGRAM_ID>,
    ]
  > &
  IInstructionWithData<Buffer>;

type Instruction = IInstruction<string>;

//
//
// XXX ixn builders

export class SinglePoolInstruction {
  static async initializePool(voteAccount: VoteAccountAddress): Promise<InitializePoolInstruction> {
    const programAddress = SINGLE_POOL_PROGRAM_ID;
    const pool = await findPoolAddress(programAddress, voteAccount);

    const type = SINGLE_POOL_INSTRUCTION_LAYOUTS.InitializePool;
    const data = encodeData(type);

    return {
      data,
      accounts: [
        { address: voteAccount, role: AccountRole.READONLY },
        { address: pool, role: AccountRole.WRITABLE },
        { address: await findPoolStakeAddress(programAddress, pool), role: AccountRole.WRITABLE },
        { address: await findPoolMintAddress(programAddress, pool), role: AccountRole.WRITABLE },
        {
          address: await findPoolStakeAuthorityAddress(programAddress, pool),
          role: AccountRole.READONLY,
        },
        {
          address: await findPoolMintAuthorityAddress(programAddress, pool),
          role: AccountRole.READONLY,
        },
        { address: SYSVAR_RENT_ID, role: AccountRole.READONLY },
        { address: SYSVAR_CLOCK_ID, role: AccountRole.READONLY },
        { address: SYSVAR_STAKE_HISTORY_ID, role: AccountRole.READONLY },
        { address: STAKE_CONFIG_ID, role: AccountRole.READONLY },
        { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
        { address: TOKEN_PROGRAM_ID, role: AccountRole.READONLY },
        { address: STAKE_PROGRAM_ID, role: AccountRole.READONLY },
      ],

      programAddress,
    };
  }
}

//
//
// XXX txn builders

export async function initialize(
  rpc: any, // XXX not exported: Rpc<SolanaRpcMethods>,
  voteAccount: VoteAccountAddress,
  payer: Base58EncodedAddress,
  skipMetadata = false,
): Promise<Transaction> {
  let transaction = { instructions: [] as any, version: 'legacy' as TransactionVersion };

  const programAddress = SINGLE_POOL_PROGRAM_ID;
  const pool = await findPoolAddress(programAddress, voteAccount);
  const stake = await findPoolStakeAddress(programAddress, pool);
  const mint = await findPoolMintAddress(programAddress, pool);

  const poolRent = await rpc.getMinimumBalanceForRentExemption(POOL_ACCOUNT_SIZE).send();
  const stakeRent = await rpc.getMinimumBalanceForRentExemption(STAKE_ACCOUNT_SIZE).send();
  const mintRent = await rpc.getMinimumBalanceForRentExemption(MINT_SIZE).send();
  const minimumDelegation = (await rpc.getStakeMinimumDelegation().send()).value;

  transaction = appendTransactionInstruction(
    SystemInstruction.transfer({
      from: payer,
      to: pool,
      lamports: poolRent,
    }),
    transaction,
  );

  transaction = appendTransactionInstruction(
    SystemInstruction.transfer({
      from: payer,
      to: stake,
      lamports: stakeRent + minimumDelegation,
    }),
    transaction,
  );

  transaction = appendTransactionInstruction(
    SystemInstruction.transfer({
      from: payer,
      to: mint,
      lamports: mintRent,
    }),
    transaction,
  );

  transaction = appendTransactionInstruction(
    await SinglePoolInstruction.initializePool(voteAccount),
    transaction,
  );

  if (!skipMetadata) {
    // TODO
  }

  return transaction;
}

//
//
// XXX test fn

async function main() {
  const transport = createDefaultRpcTransport({ url: 'http://127.0.0.1:8899' });
  const rpc = createSolanaRpc({ transport });

  const payer = await generateKeyPair();
  const payerAddress = await getBase58EncodedAddressFromPublicKey(payer.publicKey);
  await rpc.requestAirdrop(payerAddress, BigInt(100000000000) as any).send();

  await new Promise((r) => setTimeout(r, 3000));

  const voteAccount = 'KRAKEnMdmT4EfM8ykTFH6yLoCd5vNLcQvJwF66Y2dag' as VoteAccountAddress;

  const transaction0 = await initialize(rpc, voteAccount, payerAddress);
  const transaction1 = setTransactionFeePayer(payerAddress, transaction0);
  const blockhash = (await rpc.getLatestBlockhash().send()).value;
  const transaction2 = setTransactionLifetimeUsingBlockhash(blockhash, transaction1);
  const transaction3 = await signTransaction(payer, transaction2);
  console.log('transaction:', transaction3);
  for (let i = 0; i < 4; i++) {
    console.log(`\ninstruction ${i} accounts:`, transaction3.instructions[i].accounts);
  }

  const rawTransaction = getBase64EncodedWireTransaction(transaction3);
  console.log('\nraw transaction:', rawTransaction);

  //await rpc.sendTransaction(rawTransaction, { encoding: "base64", preflightCommitment: 'confirmed' }).send();
}

await main();
