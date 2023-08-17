import { PublicKey } from '@solana/web3.js';

export const SINGLE_POOL_PROGRAM_ID = new PublicKey('3cqnsMsT6LE96pxv7GR4di5rLqHDZZbR3FbeSUeRLFqY');

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
