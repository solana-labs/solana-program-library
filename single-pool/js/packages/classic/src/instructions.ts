import type { Address } from '@solana/addresses';
import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import type { PoolAddress, VoteAccountAddress } from '@solana/spl-single-pool';
import { SinglePoolInstruction as PoolInstructionModern } from '@solana/spl-single-pool';

import { modernInstructionToLegacy } from './internal.js';

export class SinglePoolInstruction {
  static async initializePool(voteAccount: PublicKey): Promise<TransactionInstruction> {
    const instruction = await PoolInstructionModern.initializePool(
      voteAccount.toBase58() as VoteAccountAddress,
    );
    return modernInstructionToLegacy(instruction);
  }

  static async reactivatePoolStake(voteAccount: PublicKey): Promise<TransactionInstruction> {
    const instruction = await PoolInstructionModern.reactivatePoolStake(
      voteAccount.toBase58() as VoteAccountAddress,
    );
    return modernInstructionToLegacy(instruction);
  }

  static async depositStake(
    pool: PublicKey,
    userStakeAccount: PublicKey,
    userTokenAccount: PublicKey,
    userLamportAccount: PublicKey,
  ): Promise<TransactionInstruction> {
    const instruction = await PoolInstructionModern.depositStake(
      pool.toBase58() as PoolAddress,
      userStakeAccount.toBase58() as Address,
      userTokenAccount.toBase58() as Address,
      userLamportAccount.toBase58() as Address,
    );
    return modernInstructionToLegacy(instruction);
  }

  static async withdrawStake(
    pool: PublicKey,
    userStakeAccount: PublicKey,
    userStakeAuthority: PublicKey,
    userTokenAccount: PublicKey,
    tokenAmount: number | bigint,
  ): Promise<TransactionInstruction> {
    const instruction = await PoolInstructionModern.withdrawStake(
      pool.toBase58() as PoolAddress,
      userStakeAccount.toBase58() as Address,
      userStakeAuthority.toBase58() as Address,
      userTokenAccount.toBase58() as Address,
      BigInt(tokenAmount),
    );
    return modernInstructionToLegacy(instruction);
  }

  static async createTokenMetadata(
    pool: PublicKey,
    payer: PublicKey,
  ): Promise<TransactionInstruction> {
    const instruction = await PoolInstructionModern.createTokenMetadata(
      pool.toBase58() as PoolAddress,
      payer.toBase58() as Address,
    );
    return modernInstructionToLegacy(instruction);
  }

  static async updateTokenMetadata(
    voteAccount: PublicKey,
    authorizedWithdrawer: PublicKey,
    tokenName: string,
    tokenSymbol: string,
    tokenUri?: string,
  ): Promise<TransactionInstruction> {
    const instruction = await PoolInstructionModern.updateTokenMetadata(
      voteAccount.toBase58() as VoteAccountAddress,
      authorizedWithdrawer.toBase58() as Address,
      tokenName,
      tokenSymbol,
      tokenUri,
    );
    return modernInstructionToLegacy(instruction);
  }
}
