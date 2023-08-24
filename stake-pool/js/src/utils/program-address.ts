import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { Buffer } from 'buffer';
import {
  METADATA_PROGRAM_ID,
  EPHEMERAL_STAKE_SEED_PREFIX,
  TRANSIENT_STAKE_SEED_PREFIX,
} from '../constants';

/**
 * Generates the withdraw authority program address for the stake pool
 */
export async function findWithdrawAuthorityProgramAddress(
  programId: PublicKey,
  stakePoolAddress: PublicKey,
) {
  const [publicKey] = await PublicKey.findProgramAddress(
    [stakePoolAddress.toBuffer(), Buffer.from('withdraw')],
    programId,
  );
  return publicKey;
}

/**
 * Generates the stake program address for a validator's vote account
 */
export async function findStakeProgramAddress(
  programId: PublicKey,
  voteAccountAddress: PublicKey,
  stakePoolAddress: PublicKey,
) {
  const [publicKey] = await PublicKey.findProgramAddress(
    [voteAccountAddress.toBuffer(), stakePoolAddress.toBuffer()],
    programId,
  );
  return publicKey;
}

/**
 * Generates the stake program address for a validator's vote account
 */
export async function findTransientStakeProgramAddress(
  programId: PublicKey,
  voteAccountAddress: PublicKey,
  stakePoolAddress: PublicKey,
  seed: BN,
) {
  const [publicKey] = await PublicKey.findProgramAddress(
    [
      TRANSIENT_STAKE_SEED_PREFIX,
      voteAccountAddress.toBuffer(),
      stakePoolAddress.toBuffer(),
      seed.toBuffer('le', 8),
    ],
    programId,
  );
  return publicKey;
}

/**
 * Generates the ephemeral program address for stake pool redelegation
 */
export async function findEphemeralStakeProgramAddress(
  programId: PublicKey,
  stakePoolAddress: PublicKey,
  seed: BN,
) {
  const [publicKey] = await PublicKey.findProgramAddress(
    [EPHEMERAL_STAKE_SEED_PREFIX, stakePoolAddress.toBuffer(), seed.toBuffer('le', 8)],
    programId,
  );
  return publicKey;
}

/**
 * Generates the metadata program address for the stake pool
 */
export function findMetadataAddress(stakePoolMintAddress: PublicKey) {
  const [publicKey] = PublicKey.findProgramAddressSync(
    [Buffer.from('metadata'), METADATA_PROGRAM_ID.toBuffer(), stakePoolMintAddress.toBuffer()],
    METADATA_PROGRAM_ID,
  );
  return publicKey;
}
