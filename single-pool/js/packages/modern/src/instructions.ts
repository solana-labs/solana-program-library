import { getAddressCodec, Address } from '@solana/addresses';
import {
  ReadonlySignerAccount,
  ReadonlyAccount,
  IInstructionWithAccounts,
  IInstructionWithData,
  WritableAccount,
  WritableSignerAccount,
  IInstruction,
  AccountRole,
} from '@solana/instructions';

import {
  PoolMintAuthorityAddress,
  PoolMintAddress,
  PoolMplAuthorityAddress,
  PoolStakeAuthorityAddress,
  PoolStakeAddress,
  findMplMetadataAddress,
  findPoolMplAuthorityAddress,
  findPoolAddress,
  VoteAccountAddress,
  PoolAddress,
  findPoolStakeAddress,
  findPoolMintAddress,
  findPoolMintAuthorityAddress,
  findPoolStakeAuthorityAddress,
  SINGLE_POOL_PROGRAM_ID,
} from './addresses.js';
import { MPL_METADATA_PROGRAM_ID } from './internal.js';
import {
  SYSTEM_PROGRAM_ID,
  SYSVAR_RENT_ID,
  SYSVAR_CLOCK_ID,
  STAKE_PROGRAM_ID,
  SYSVAR_STAKE_HISTORY_ID,
  STAKE_CONFIG_ID,
  TOKEN_PROGRAM_ID,
  u32,
  u64,
} from './quarantine.js';

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
  IInstructionWithData<Uint8Array>;

type ReactivatePoolStakeInstruction = IInstruction<typeof SINGLE_POOL_PROGRAM_ID> &
  IInstructionWithAccounts<
    [
      ReadonlyAccount<VoteAccountAddress>,
      ReadonlyAccount<PoolAddress>,
      WritableAccount<PoolStakeAddress>,
      ReadonlyAccount<PoolStakeAuthorityAddress>,
      ReadonlyAccount<typeof SYSVAR_CLOCK_ID>,
      ReadonlyAccount<typeof SYSVAR_STAKE_HISTORY_ID>,
      ReadonlyAccount<typeof STAKE_CONFIG_ID>,
      ReadonlyAccount<typeof STAKE_PROGRAM_ID>,
    ]
  > &
  IInstructionWithData<Uint8Array>;

type DepositStakeInstruction = IInstruction<typeof SINGLE_POOL_PROGRAM_ID> &
  IInstructionWithAccounts<
    [
      ReadonlyAccount<PoolAddress>,
      WritableAccount<PoolStakeAddress>,
      WritableAccount<PoolMintAddress>,
      ReadonlyAccount<PoolStakeAuthorityAddress>,
      ReadonlyAccount<PoolMintAuthorityAddress>,
      WritableAccount<Address>, // user stake
      WritableAccount<Address>, // user token
      WritableAccount<Address>, // user lamport
      ReadonlyAccount<typeof SYSVAR_CLOCK_ID>,
      ReadonlyAccount<typeof SYSVAR_STAKE_HISTORY_ID>,
      ReadonlyAccount<typeof TOKEN_PROGRAM_ID>,
      ReadonlyAccount<typeof STAKE_PROGRAM_ID>,
    ]
  > &
  IInstructionWithData<Uint8Array>;

type WithdrawStakeInstruction = IInstruction<typeof SINGLE_POOL_PROGRAM_ID> &
  IInstructionWithAccounts<
    [
      ReadonlyAccount<PoolAddress>,
      WritableAccount<PoolStakeAddress>,
      WritableAccount<PoolMintAddress>,
      ReadonlyAccount<PoolStakeAuthorityAddress>,
      ReadonlyAccount<PoolMintAuthorityAddress>,
      WritableAccount<Address>, // user stake
      WritableAccount<Address>, // user token
      ReadonlyAccount<typeof SYSVAR_CLOCK_ID>,
      ReadonlyAccount<typeof TOKEN_PROGRAM_ID>,
      ReadonlyAccount<typeof STAKE_PROGRAM_ID>,
    ]
  > &
  IInstructionWithData<Uint8Array>;

type CreateTokenMetadataInstruction = IInstruction<typeof SINGLE_POOL_PROGRAM_ID> &
  IInstructionWithAccounts<
    [
      ReadonlyAccount<PoolAddress>,
      ReadonlyAccount<PoolMintAddress>,
      ReadonlyAccount<PoolMintAuthorityAddress>,
      ReadonlyAccount<PoolMplAuthorityAddress>,
      WritableSignerAccount<Address>, // mpl payer
      WritableAccount<Address>, // mpl account
      ReadonlyAccount<typeof MPL_METADATA_PROGRAM_ID>,
      ReadonlyAccount<typeof SYSTEM_PROGRAM_ID>,
    ]
  > &
  IInstructionWithData<Uint8Array>;

type UpdateTokenMetadataInstruction = IInstruction<typeof SINGLE_POOL_PROGRAM_ID> &
  IInstructionWithAccounts<
    [
      ReadonlyAccount<VoteAccountAddress>,
      ReadonlyAccount<PoolAddress>,
      ReadonlyAccount<PoolMplAuthorityAddress>,
      ReadonlySignerAccount<Address>, // authorized withdrawer
      WritableAccount<Address>, // mpl account
      ReadonlyAccount<typeof MPL_METADATA_PROGRAM_ID>,
    ]
  > &
  IInstructionWithData<Uint8Array>;

const enum SinglePoolInstructionType {
  InitializePool = 0,
  ReactivatePoolStake,
  DepositStake,
  WithdrawStake,
  CreateTokenMetadata,
  UpdateTokenMetadata,
}

export const SinglePoolInstruction = {
  initializePool: initializePoolInstruction,
  reactivatePoolStake: reactivatePoolStakeInstruction,
  depositStake: depositStakeInstruction,
  withdrawStake: withdrawStakeInstruction,
  createTokenMetadata: createTokenMetadataInstruction,
  updateTokenMetadata: updateTokenMetadataInstruction,
};

export async function initializePoolInstruction(
  voteAccount: VoteAccountAddress,
): Promise<InitializePoolInstruction> {
  const programAddress = SINGLE_POOL_PROGRAM_ID;
  const pool = await findPoolAddress(programAddress, voteAccount);
  const [stake, mint, stakeAuthority, mintAuthority] = await Promise.all([
    findPoolStakeAddress(programAddress, pool),
    findPoolMintAddress(programAddress, pool),
    findPoolStakeAuthorityAddress(programAddress, pool),
    findPoolMintAuthorityAddress(programAddress, pool),
  ]);

  const data = new Uint8Array([SinglePoolInstructionType.InitializePool]);

  return {
    data,
    accounts: [
      { address: voteAccount, role: AccountRole.READONLY },
      { address: pool, role: AccountRole.WRITABLE },
      { address: stake, role: AccountRole.WRITABLE },
      { address: mint, role: AccountRole.WRITABLE },
      { address: stakeAuthority, role: AccountRole.READONLY },
      { address: mintAuthority, role: AccountRole.READONLY },
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

export async function reactivatePoolStakeInstruction(
  voteAccount: VoteAccountAddress,
): Promise<ReactivatePoolStakeInstruction> {
  const programAddress = SINGLE_POOL_PROGRAM_ID;
  const pool = await findPoolAddress(programAddress, voteAccount);
  const [stake, stakeAuthority] = await Promise.all([
    findPoolStakeAddress(programAddress, pool),
    findPoolStakeAuthorityAddress(programAddress, pool),
  ]);

  const data = new Uint8Array([SinglePoolInstructionType.ReactivatePoolStake]);

  return {
    data,
    accounts: [
      { address: voteAccount, role: AccountRole.READONLY },
      { address: pool, role: AccountRole.READONLY },
      { address: stake, role: AccountRole.WRITABLE },
      { address: stakeAuthority, role: AccountRole.READONLY },
      { address: SYSVAR_CLOCK_ID, role: AccountRole.READONLY },
      { address: SYSVAR_STAKE_HISTORY_ID, role: AccountRole.READONLY },
      { address: STAKE_CONFIG_ID, role: AccountRole.READONLY },
      { address: STAKE_PROGRAM_ID, role: AccountRole.READONLY },
    ],
    programAddress,
  };
}

export async function depositStakeInstruction(
  pool: PoolAddress,
  userStakeAccount: Address,
  userTokenAccount: Address,
  userLamportAccount: Address,
): Promise<DepositStakeInstruction> {
  const programAddress = SINGLE_POOL_PROGRAM_ID;
  const [stake, mint, stakeAuthority, mintAuthority] = await Promise.all([
    findPoolStakeAddress(programAddress, pool),
    findPoolMintAddress(programAddress, pool),
    findPoolStakeAuthorityAddress(programAddress, pool),
    findPoolMintAuthorityAddress(programAddress, pool),
  ]);

  const data = new Uint8Array([SinglePoolInstructionType.DepositStake]);

  return {
    data,
    accounts: [
      { address: pool, role: AccountRole.READONLY },
      { address: stake, role: AccountRole.WRITABLE },
      { address: mint, role: AccountRole.WRITABLE },
      { address: stakeAuthority, role: AccountRole.READONLY },
      { address: mintAuthority, role: AccountRole.READONLY },
      { address: userStakeAccount, role: AccountRole.WRITABLE },
      { address: userTokenAccount, role: AccountRole.WRITABLE },
      { address: userLamportAccount, role: AccountRole.WRITABLE },
      { address: SYSVAR_CLOCK_ID, role: AccountRole.READONLY },
      { address: SYSVAR_STAKE_HISTORY_ID, role: AccountRole.READONLY },
      { address: TOKEN_PROGRAM_ID, role: AccountRole.READONLY },
      { address: STAKE_PROGRAM_ID, role: AccountRole.READONLY },
    ],
    programAddress,
  };
}

export async function withdrawStakeInstruction(
  pool: PoolAddress,
  userStakeAccount: Address,
  userStakeAuthority: Address,
  userTokenAccount: Address,
  tokenAmount: bigint,
): Promise<WithdrawStakeInstruction> {
  const programAddress = SINGLE_POOL_PROGRAM_ID;
  const [stake, mint, stakeAuthority, mintAuthority] = await Promise.all([
    findPoolStakeAddress(programAddress, pool),
    findPoolMintAddress(programAddress, pool),
    findPoolStakeAuthorityAddress(programAddress, pool),
    findPoolMintAuthorityAddress(programAddress, pool),
  ]);

  const { encode } = getAddressCodec();
  const data = new Uint8Array([
    SinglePoolInstructionType.WithdrawStake,
    ...encode(userStakeAuthority),
    ...u64(tokenAmount),
  ]);

  return {
    data,
    accounts: [
      { address: pool, role: AccountRole.READONLY },
      { address: stake, role: AccountRole.WRITABLE },
      { address: mint, role: AccountRole.WRITABLE },
      { address: stakeAuthority, role: AccountRole.READONLY },
      { address: mintAuthority, role: AccountRole.READONLY },
      { address: userStakeAccount, role: AccountRole.WRITABLE },
      { address: userTokenAccount, role: AccountRole.WRITABLE },
      { address: SYSVAR_CLOCK_ID, role: AccountRole.READONLY },
      { address: TOKEN_PROGRAM_ID, role: AccountRole.READONLY },
      { address: STAKE_PROGRAM_ID, role: AccountRole.READONLY },
    ],
    programAddress,
  };
}

export async function createTokenMetadataInstruction(
  pool: PoolAddress,
  payer: Address,
): Promise<CreateTokenMetadataInstruction> {
  const programAddress = SINGLE_POOL_PROGRAM_ID;
  const mint = await findPoolMintAddress(programAddress, pool);
  const [mintAuthority, mplAuthority, mplMetadata] = await Promise.all([
    findPoolMintAuthorityAddress(programAddress, pool),
    findPoolMplAuthorityAddress(programAddress, pool),
    findMplMetadataAddress(mint),
  ]);

  const data = new Uint8Array([SinglePoolInstructionType.CreateTokenMetadata]);

  return {
    data,
    accounts: [
      { address: pool, role: AccountRole.READONLY },
      { address: mint, role: AccountRole.READONLY },
      { address: mintAuthority, role: AccountRole.READONLY },
      { address: mplAuthority, role: AccountRole.READONLY },
      { address: payer, role: AccountRole.WRITABLE_SIGNER },
      { address: mplMetadata, role: AccountRole.WRITABLE },
      { address: MPL_METADATA_PROGRAM_ID, role: AccountRole.READONLY },
      { address: SYSTEM_PROGRAM_ID, role: AccountRole.READONLY },
    ],
    programAddress,
  };
}

export async function updateTokenMetadataInstruction(
  voteAccount: VoteAccountAddress,
  authorizedWithdrawer: Address,
  tokenName: string,
  tokenSymbol: string,
  tokenUri?: string,
): Promise<UpdateTokenMetadataInstruction> {
  const programAddress = SINGLE_POOL_PROGRAM_ID;
  tokenUri = tokenUri || '';

  if (tokenName.length > 32) {
    throw 'maximum token name length is 32 characters';
  }

  if (tokenSymbol.length > 10) {
    throw 'maximum token symbol length is 10 characters';
  }

  if (tokenUri.length > 200) {
    throw 'maximum token uri length is 200 characters';
  }

  const pool = await findPoolAddress(programAddress, voteAccount);
  const [mint, mplAuthority] = await Promise.all([
    findPoolMintAddress(programAddress, pool),
    findPoolMplAuthorityAddress(programAddress, pool),
  ]);
  const mplMetadata = await findMplMetadataAddress(mint);

  const text = new TextEncoder();
  const data = new Uint8Array([
    SinglePoolInstructionType.UpdateTokenMetadata,
    ...u32(tokenName.length),
    ...text.encode(tokenName),
    ...u32(tokenSymbol.length),
    ...text.encode(tokenSymbol),
    ...u32(tokenUri.length),
    ...text.encode(tokenUri),
  ]);

  return {
    data,
    accounts: [
      { address: voteAccount, role: AccountRole.READONLY },
      { address: pool, role: AccountRole.READONLY },
      { address: mplAuthority, role: AccountRole.READONLY },
      { address: authorizedWithdrawer, role: AccountRole.READONLY_SIGNER },
      { address: mplMetadata, role: AccountRole.WRITABLE },
      { address: MPL_METADATA_PROGRAM_ID, role: AccountRole.READONLY },
    ],
    programAddress,
  };
}
