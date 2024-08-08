import type { Address } from '@solana/addresses';
import { PublicKey, Connection } from '@solana/web3.js';
import type { PoolAddress, VoteAccountAddress } from '@solana/spl-single-pool';
import { SinglePoolProgram as PoolProgramModern } from '@solana/spl-single-pool';

import { paramsToModern, modernTransactionToLegacy, rpc } from './internal.js';

interface DepositParams {
  connection: Connection;
  pool: PublicKey;
  userWallet: PublicKey;
  userStakeAccount?: PublicKey;
  depositFromDefaultAccount?: boolean;
  userTokenAccount?: PublicKey;
  userLamportAccount?: PublicKey;
  userWithdrawAuthority?: PublicKey;
}

interface WithdrawParams {
  connection: Connection;
  pool: PublicKey;
  userWallet: PublicKey;
  userStakeAccount: PublicKey;
  tokenAmount: number | bigint;
  createStakeAccount?: boolean;
  userStakeAuthority?: PublicKey;
  userTokenAccount?: PublicKey;
  userTokenAuthority?: PublicKey;
}

export class SinglePoolProgram {
  static programId: PublicKey = new PublicKey(PoolProgramModern.programAddress);
  static space: number = Number(PoolProgramModern.space);

  static async initialize(
    connection: Connection,
    voteAccount: PublicKey,
    payer: PublicKey,
    skipMetadata = false,
  ) {
    const modernTransaction = await PoolProgramModern.initialize(
      rpc(connection),
      voteAccount.toBase58() as VoteAccountAddress,
      payer.toBase58() as Address,
      skipMetadata,
    );

    return modernTransactionToLegacy(modernTransaction);
  }

  static async reactivatePoolStake(connection: Connection, voteAccount: PublicKey) {
    const modernTransaction = await PoolProgramModern.reactivatePoolStake(
      voteAccount.toBase58() as VoteAccountAddress,
    );

    return modernTransactionToLegacy(modernTransaction);
  }

  static async deposit(params: DepositParams) {
    const modernParams = paramsToModern(params);
    const modernTransaction = await PoolProgramModern.deposit(modernParams);

    return modernTransactionToLegacy(modernTransaction);
  }

  static async withdraw(params: WithdrawParams) {
    const modernParams = paramsToModern(params);
    const modernTransaction = await PoolProgramModern.withdraw(modernParams);

    return modernTransactionToLegacy(modernTransaction);
  }

  static async createTokenMetadata(pool: PublicKey, payer: PublicKey) {
    const modernTransaction = await PoolProgramModern.createTokenMetadata(
      pool.toBase58() as PoolAddress,
      payer.toBase58() as Address,
    );

    return modernTransactionToLegacy(modernTransaction);
  }

  static async updateTokenMetadata(
    voteAccount: PublicKey,
    authorizedWithdrawer: PublicKey,
    name: string,
    symbol: string,
    uri?: string,
  ) {
    const modernTransaction = await PoolProgramModern.updateTokenMetadata(
      voteAccount.toBase58() as VoteAccountAddress,
      authorizedWithdrawer.toBase58() as Address,
      name,
      symbol,
      uri,
    );

    return modernTransactionToLegacy(modernTransaction);
  }

  static async createAndDelegateUserStake(
    connection: Connection,
    voteAccount: PublicKey,
    userWallet: PublicKey,
    stakeAmount: number | bigint,
  ) {
    const modernTransaction = await PoolProgramModern.createAndDelegateUserStake(
      rpc(connection),
      voteAccount.toBase58() as VoteAccountAddress,
      userWallet.toBase58() as Address,
      BigInt(stakeAmount),
    );

    return modernTransactionToLegacy(modernTransaction);
  }
}
