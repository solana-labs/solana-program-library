import type { Address } from '@solana/addresses';
import { PublicKey } from '@solana/web3.js';
import type { PoolAddress, VoteAccountAddress } from '@solana/spl-single-pool';
import {
  findPoolAddress as findPoolModern,
  findPoolStakeAddress as findStakeModern,
  findPoolMintAddress as findMintModern,
  findPoolStakeAuthorityAddress as findStakeAuthorityModern,
  findPoolMintAuthorityAddress as findMintAuthorityModern,
  findPoolMplAuthorityAddress as findMplAuthorityModern,
  findDefaultDepositAccountAddress as findDefaultDepositModern,
} from '@solana/spl-single-pool';

export async function findPoolAddress(programId: PublicKey, voteAccountAddress: PublicKey) {
  return new PublicKey(
    await findPoolModern(
      programId.toBase58() as Address,
      voteAccountAddress.toBase58() as VoteAccountAddress,
    ),
  );
}

export async function findPoolStakeAddress(programId: PublicKey, poolAddress: PublicKey) {
  return new PublicKey(
    await findStakeModern(programId.toBase58() as Address, poolAddress.toBase58() as PoolAddress),
  );
}

export async function findPoolMintAddress(programId: PublicKey, poolAddress: PublicKey) {
  return new PublicKey(
    await findMintModern(programId.toBase58() as Address, poolAddress.toBase58() as PoolAddress),
  );
}

export async function findPoolStakeAuthorityAddress(programId: PublicKey, poolAddress: PublicKey) {
  return new PublicKey(
    await findStakeAuthorityModern(
      programId.toBase58() as Address,
      poolAddress.toBase58() as PoolAddress,
    ),
  );
}

export async function findPoolMintAuthorityAddress(programId: PublicKey, poolAddress: PublicKey) {
  return new PublicKey(
    await findMintAuthorityModern(
      programId.toBase58() as Address,
      poolAddress.toBase58() as PoolAddress,
    ),
  );
}

export async function findPoolMplAuthorityAddress(programId: PublicKey, poolAddress: PublicKey) {
  return new PublicKey(
    await findMplAuthorityModern(
      programId.toBase58() as Address,
      poolAddress.toBase58() as PoolAddress,
    ),
  );
}

export async function findDefaultDepositAccountAddress(
  poolAddress: PublicKey,
  userWallet: PublicKey,
) {
  return new PublicKey(
    await findDefaultDepositModern(
      poolAddress.toBase58() as PoolAddress,
      userWallet.toBase58() as Address,
    ),
  );
}
