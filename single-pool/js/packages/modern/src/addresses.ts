import {
  address,
  getAddressCodec,
  Base58EncodedAddress,
  getProgramDerivedAddress,
  createAddressWithSeed,
} from '@solana/web3.js';

import { MPL_METADATA_PROGRAM_ID } from './internal.js';
import { STAKE_PROGRAM_ID } from './quarantine.js';

export const SINGLE_POOL_PROGRAM_ID = address('SVSPxpvHdN29nkVg9rPapPNDddN5DipNLRUFhyjFThE');

export type VoteAccountAddress<TAddress extends string = string> =
  Base58EncodedAddress<TAddress> & {
    readonly __voteAccountAddress: unique symbol;
  };

export type PoolAddress<TAddress extends string = string> = Base58EncodedAddress<TAddress> & {
  readonly __poolAddress: unique symbol;
};

export type PoolStakeAddress<TAddress extends string = string> = Base58EncodedAddress<TAddress> & {
  readonly __poolStakeAddress: unique symbol;
};

export type PoolMintAddress<TAddress extends string = string> = Base58EncodedAddress<TAddress> & {
  readonly __poolMintAddress: unique symbol;
};

export type PoolStakeAuthorityAddress<TAddress extends string = string> =
  Base58EncodedAddress<TAddress> & {
    readonly __poolStakeAuthorityAddress: unique symbol;
  };

export type PoolMintAuthorityAddress<TAddress extends string = string> =
  Base58EncodedAddress<TAddress> & {
    readonly __poolMintAuthorityAddress: unique symbol;
  };

export type PoolMplAuthorityAddress<TAddress extends string = string> =
  Base58EncodedAddress<TAddress> & {
    readonly __poolMplAuthorityAddress: unique symbol;
  };

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
  const { serialize } = getAddressCodec();
  const [pda] = await getProgramDerivedAddress({
    programAddress: programId,
    seeds: [prefix, serialize(baseAddress)],
  });

  return pda;
}

export async function findDefaultDepositAccountAddress(
  poolAddress: PoolAddress,
  userWallet: Base58EncodedAddress,
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
  const { serialize } = getAddressCodec();
  const [pda] = await getProgramDerivedAddress({
    programAddress: MPL_METADATA_PROGRAM_ID,
    seeds: ['metadata', serialize(MPL_METADATA_PROGRAM_ID), serialize(poolMintAddress)],
  });

  return pda;
}
