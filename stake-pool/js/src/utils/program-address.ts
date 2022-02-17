import {PublicKey} from '@solana/web3.js';
import BN from 'bn.js';
import {TRANSIENT_STAKE_SEED_PREFIX} from '../constants';

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
      new Uint8Array(seed.toArray('le', 8)),
    ],
    programId,
  );
  return publicKey;
}
