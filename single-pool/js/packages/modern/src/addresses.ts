import {
  address,
  getAddressCodec,
  getProgramDerivedAddress,
  createAddressWithSeed,
  Address,
} from '@solana/addresses';

import { MPL_METADATA_PROGRAM_ID } from './internal.js';
import { STAKE_PROGRAM_ID } from './quarantine.js';

export const SINGLE_POOL_PROGRAM_ID = address('SVSPxpvHdN29nkVg9rPapPNDddN5DipNLRUFhyjFThE');

export type VoteAccountAddress<TAddress extends string = string> = Address<TAddress> & {
  readonly __voteAccountAddress: unique symbol;
};

export type PoolAddress<TAddress extends string = string> = Address<TAddress> & {
  readonly __poolAddress: unique symbol;
};

export type PoolStakeAddress<TAddress extends string = string> = Address<TAddress> & {
  readonly __poolStakeAddress: unique symbol;
};

export type PoolMintAddress<TAddress extends string = string> = Address<TAddress> & {
  readonly __poolMintAddress: unique symbol;
};

export type PoolStakeAuthorityAddress<TAddress extends string = string> = Address<TAddress> & {
  readonly __poolStakeAuthorityAddress: unique symbol;
};

export type PoolMintAuthorityAddress<TAddress extends string = string> = Address<TAddress> & {
  readonly __poolMintAuthorityAddress: unique symbol;
};

export type PoolMplAuthorityAddress<TAddress extends string = string> = Address<TAddress> & {
  readonly __poolMplAuthorityAddress: unique symbol;
};

export async function findPoolAddress(
  programId: Address,
  voteAccountAddress: VoteAccountAddress,
): Promise<PoolAddress> {
  return (await findPda(programId, voteAccountAddress, 'pool')) as PoolAddress;
}

export async function findPoolStakeAddress(
  programId: Address,
  poolAddress: PoolAddress,
): Promise<PoolStakeAddress> {
  return (await findPda(programId, poolAddress, 'stake')) as PoolStakeAddress;
}

export async function findPoolMintAddress(
  programId: Address,
  poolAddress: PoolAddress,
): Promise<PoolMintAddress> {
  return (await findPda(programId, poolAddress, 'mint')) as PoolMintAddress;
}

export async function findPoolStakeAuthorityAddress(
  programId: Address,
  poolAddress: PoolAddress,
): Promise<PoolStakeAuthorityAddress> {
  return (await findPda(programId, poolAddress, 'stake_authority')) as PoolStakeAuthorityAddress;
}

export async function findPoolMintAuthorityAddress(
  programId: Address,
  poolAddress: PoolAddress,
): Promise<PoolMintAuthorityAddress> {
  return (await findPda(programId, poolAddress, 'mint_authority')) as PoolMintAuthorityAddress;
}

export async function findPoolMplAuthorityAddress(
  programId: Address,
  poolAddress: PoolAddress,
): Promise<PoolMplAuthorityAddress> {
  return (await findPda(programId, poolAddress, 'mpl_authority')) as PoolMplAuthorityAddress;
}

async function findPda(programId: Address, baseAddress: Address, prefix: string) {
  const { encode } = getAddressCodec();
  const [pda] = await getProgramDerivedAddress({
    programAddress: programId,
    seeds: [prefix, encode(baseAddress)],
  });

  return pda;
}

export async function findDefaultDepositAccountAddress(
  poolAddress: PoolAddress,
  userWallet: Address,
) {
  return createAddressWithSeed({
    baseAddress: userWallet,
    seed: defaultDepositAccountSeed(poolAddress),
    programAddress: STAKE_PROGRAM_ID,
  });
}

export function defaultDepositAccountSeed(poolAddress: PoolAddress): string {
  return 'svsp' + poolAddress.slice(0, 28);
}

export async function findMplMetadataAddress(poolMintAddress: PoolMintAddress) {
  const { encode } = getAddressCodec();
  const [pda] = await getProgramDerivedAddress({
    programAddress: MPL_METADATA_PROGRAM_ID,
    seeds: ['metadata', encode(MPL_METADATA_PROGRAM_ID), encode(poolMintAddress)],
  });

  return pda;
}
