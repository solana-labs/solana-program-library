import { PublicKey, TransactionInstruction } from '@solana/web3.js';
import { SinglePoolInstruction as PoolInstructionModern } from '@solana/spl-single-pool';

import { modernInstructionToLegacy } from './internal.js';

export class SinglePoolInstruction {
  static async initializePool(voteAccount: PublicKey): Promise<TransactionInstruction> {
    const instruction = await PoolInstructionModern.initializePool(voteAccount.toBase58());
    return modernInstructionToLegacy(instruction);
  }

  static async reactivatePoolStake(voteAccount: PublicKey): Promise<TransactionInstruction> {
    const instruction = await PoolInstructionModern.reactivatePoolStake(voteAccount.toBase58());
    return modernInstructionToLegacy(instruction);
  }

  static async depositStake(
    pool: PublicKey,
    userStakeAccount: PublicKey,
    userTokenAccount: PublicKey,
    userLamportAccount: PublicKey,
  ): Promise<TransactionInstruction> {
    const instruction = await PoolInstructionModern.depositStake(
      pool.toBase58(),
      userStakeAccount.toBase58(),
      userTokenAccount.toBase58(),
      userLamportAccount.toBase58(),
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
      pool.toBase58(),
      userStakeAccount.toBase58(),
      userStakeAuthority.toBase58(),
      userTokenAccount.toBase58(),
      BigInt(tokenAmount),
    );
    return modernInstructionToLegacy(instruction);
  }

  static async createTokenMetadata(
    pool: PublicKey,
    payer: PublicKey,
  ): Promise<TransactionInstruction> {
    const instruction = await PoolInstructionModern.createTokenMetadata(
      pool.toBase58(),
      payer.toBase58(),
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
      voteAccount.toBase58(),
      authorizedWithdrawer.toBase58(),
      tokenName,
      tokenSymbol,
      tokenUri,
    );
    return modernInstructionToLegacy(instruction);
  }
}
