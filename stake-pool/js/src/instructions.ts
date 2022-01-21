/**
 * Based on https://github.com/solana-labs/solana-web3.js/blob/master/src/stake-program.ts
 */
import {
  encodeData,
  decodeData,
  InstructionType,
} from './copied-from-solana-web3/instruction';
import {
  PublicKey,
  TransactionInstruction,
  StakeProgram,
  SystemProgram,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_STAKE_HISTORY_PUBKEY,
} from '@solana/web3.js';
import {struct, u8, nu64} from '@solana/buffer-layout';
import {TOKEN_PROGRAM_ID} from '@solana/spl-token';
import {STAKE_POOL_PROGRAM_ID} from './constants';

/**
 * An enumeration of valid StakePoolInstructionType's
 */
export type StakePoolInstructionType =
  | 'DepositStake'
  | 'DepositSol'
  | 'WithdrawStake'
  | 'WithdrawSol';

/**
 * An enumeration of valid stake InstructionType's
 * @internal
 */
export const STAKE_POOL_INSTRUCTION_LAYOUTS: {
  [type in StakePoolInstructionType]: InstructionType;
} = Object.freeze({
  DepositStake: {
    index: 9,
    layout: struct([u8('instruction') as any]), // NOTE do this better
  },
  /// Withdraw the token from the pool at the current ratio.
  WithdrawStake: {
    index: 10,
    layout: struct([
      u8('instruction') as any, // NOTE do this better
      nu64('poolTokens'),
    ]),
  },
  /// Deposit SOL directly into the pool's reserve account. The output is a "pool" token
  /// representing ownership into the pool. Inputs are converted to the current ratio.
  DepositSol: {
    index: 14,
    layout: struct([
      u8('instruction') as any, // NOTE do this better
      nu64('lamports'),
    ]),
  },
  /// Withdraw SOL directly from the pool's reserve account. Fails if the
  /// reserve does not have enough SOL.
  WithdrawSol: {
    index: 16,
    layout: struct([u8('instruction') as any, nu64('poolTokens')]),
  },
});

/**
 * Deposit stake pool instruction params
 */
export type DepositStakeParams = {
  stakePool: PublicKey;
  validatorList: PublicKey;
  depositAuthority: PublicKey;
  withdrawAuthority: PublicKey;
  depositStake: PublicKey;
  validatorStake: PublicKey;
  reserveStake: PublicKey;
  destinationPoolAccount: PublicKey;
  managerFeeAccount: PublicKey;
  referralPoolAccount: PublicKey;
  poolMint: PublicKey;
};

/**
 * Withdraw stake pool instruction params
 */
export type WithdrawStakeParams = {
  stakePool: PublicKey;
  validatorList: PublicKey;
  withdrawAuthority: PublicKey;
  validatorStake: PublicKey;
  destinationStake: PublicKey;
  destinationStakeAuthority: PublicKey;
  sourceTransferAuthority: PublicKey;
  sourcePoolAccount: PublicKey;
  managerFeeAccount: PublicKey;
  poolMint: PublicKey;
  poolTokens: number;
};

/**
 * Withdraw sol instruction params
 */
export type WithdrawSolParams = {
  stakePool: PublicKey;
  sourcePoolAccount: PublicKey;
  withdrawAuthority: PublicKey;
  reserveStake: PublicKey;
  destinationSystemAccount: PublicKey;
  sourceTransferAuthority: PublicKey;
  solWithdrawAuthority?: PublicKey | undefined;
  managerFeeAccount: PublicKey;
  poolMint: PublicKey;
  poolTokens: number;
};

/**
 * Deposit sol instruction params
 */
export type DepositSolParams = {
  stakePool: PublicKey;
  depositAuthority?: PublicKey | undefined;
  withdrawAuthority: PublicKey;
  reserveStake: PublicKey;
  fundingAccount: PublicKey;
  destinationPoolAccount: PublicKey;
  managerFeeAccount: PublicKey;
  referralPoolAccount: PublicKey;
  poolMint: PublicKey;
  lamports: number;
};

/**
 * Stake Pool Instruction class
 */
export class StakePoolInstruction {
  /**
   * Creates a transaction instruction to deposit SOL into a stake pool.
   */
  static depositStake(params: DepositStakeParams): TransactionInstruction {
    const {
      stakePool,
      validatorList,
      depositAuthority,
      withdrawAuthority,
      depositStake,
      validatorStake,
      reserveStake,
      destinationPoolAccount,
      managerFeeAccount,
      referralPoolAccount,
      poolMint,
    } = params;

    const type = STAKE_POOL_INSTRUCTION_LAYOUTS.DepositStake;
    const data = encodeData(type);

    const keys = [
      {pubkey: stakePool, isSigner: false, isWritable: true},
      {pubkey: validatorList, isSigner: false, isWritable: true},
      {pubkey: depositAuthority, isSigner: false, isWritable: false},
      {pubkey: withdrawAuthority, isSigner: false, isWritable: false},
      {pubkey: depositStake, isSigner: false, isWritable: true},
      {pubkey: validatorStake, isSigner: false, isWritable: true},
      {pubkey: reserveStake, isSigner: false, isWritable: true},
      {pubkey: destinationPoolAccount, isSigner: false, isWritable: true},
      {pubkey: managerFeeAccount, isSigner: false, isWritable: true},
      {pubkey: referralPoolAccount, isSigner: false, isWritable: true},
      {pubkey: poolMint, isSigner: false, isWritable: true},
      {pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false},
      {pubkey: SYSVAR_STAKE_HISTORY_PUBKEY, isSigner: false, isWritable: false},
      {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
      {pubkey: StakeProgram.programId, isSigner: false, isWritable: false},
    ];

    return new TransactionInstruction({
      programId: STAKE_POOL_PROGRAM_ID,
      keys,
      data,
    });
  }

  /**
   * Creates a transaction instruction to withdraw SOL from a stake pool.
   */
  static depositSol(params: DepositSolParams): TransactionInstruction {
    const {
      stakePool,
      withdrawAuthority,
      depositAuthority,
      reserveStake,
      fundingAccount,
      destinationPoolAccount,
      managerFeeAccount,
      referralPoolAccount,
      poolMint,
      lamports,
    } = params;

    const type = STAKE_POOL_INSTRUCTION_LAYOUTS.DepositSol;
    const data = encodeData(type, {lamports});

    const keys = [
      {pubkey: stakePool, isSigner: false, isWritable: true},
      {pubkey: withdrawAuthority, isSigner: false, isWritable: false},
      {pubkey: reserveStake, isSigner: false, isWritable: true},
      {pubkey: fundingAccount, isSigner: true, isWritable: true},
      {pubkey: destinationPoolAccount, isSigner: false, isWritable: true},
      {pubkey: managerFeeAccount, isSigner: false, isWritable: true},
      {pubkey: referralPoolAccount, isSigner: false, isWritable: true},
      {pubkey: poolMint, isSigner: false, isWritable: true},
      {pubkey: SystemProgram.programId, isSigner: false, isWritable: false},
      {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
    ];

    if (depositAuthority) {
      keys.push({
        pubkey: depositAuthority,
        isSigner: true,
        isWritable: false,
      });
    }

    return new TransactionInstruction({
      programId: STAKE_POOL_PROGRAM_ID,
      keys,
      data,
    });
  }

  /**
   * Creates a transaction instruction to withdraw SOL from a stake pool.
   */
  static withdrawStake(params: WithdrawStakeParams): TransactionInstruction {
    const {
      stakePool,
      validatorList,
      withdrawAuthority,
      validatorStake,
      destinationStake,
      destinationStakeAuthority,
      sourceTransferAuthority,
      sourcePoolAccount,
      managerFeeAccount,
      poolMint,
      poolTokens,
    } = params;

    const type = STAKE_POOL_INSTRUCTION_LAYOUTS.WithdrawStake;
    const data = encodeData(type, {poolTokens});

    const keys = [
      {pubkey: stakePool, isSigner: false, isWritable: true},
      {pubkey: validatorList, isSigner: false, isWritable: true},
      {pubkey: withdrawAuthority, isSigner: false, isWritable: false},
      {pubkey: validatorStake, isSigner: false, isWritable: true},
      {pubkey: destinationStake, isSigner: false, isWritable: true},
      {pubkey: destinationStakeAuthority, isSigner: false, isWritable: false},
      {pubkey: sourceTransferAuthority, isSigner: true, isWritable: false},
      {pubkey: sourcePoolAccount, isSigner: false, isWritable: true},
      {pubkey: managerFeeAccount, isSigner: false, isWritable: true},
      {pubkey: poolMint, isSigner: false, isWritable: true},
      {pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false},
      {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
      {pubkey: StakeProgram.programId, isSigner: false, isWritable: false},
    ];

    return new TransactionInstruction({
      programId: STAKE_POOL_PROGRAM_ID,
      keys,
      data,
    });
  }

  /**
   * Creates a transaction instruction to withdraw SOL from a stake pool.
   */
  static withdrawSol(params: WithdrawSolParams): TransactionInstruction {
    const {
      stakePool,
      withdrawAuthority,
      sourceTransferAuthority,
      sourcePoolAccount,
      reserveStake,
      destinationSystemAccount,
      managerFeeAccount,
      solWithdrawAuthority,
      poolMint,
      poolTokens,
    } = params;

    const type = STAKE_POOL_INSTRUCTION_LAYOUTS.WithdrawSol;
    const data = encodeData(type, {poolTokens});

    const keys = [
      {pubkey: stakePool, isSigner: false, isWritable: true},
      {pubkey: withdrawAuthority, isSigner: false, isWritable: false},
      {pubkey: sourceTransferAuthority, isSigner: true, isWritable: false},
      {pubkey: sourcePoolAccount, isSigner: false, isWritable: true},
      {pubkey: reserveStake, isSigner: false, isWritable: true},
      {pubkey: destinationSystemAccount, isSigner: false, isWritable: true},
      {pubkey: managerFeeAccount, isSigner: false, isWritable: true},
      {pubkey: poolMint, isSigner: false, isWritable: true},
      {pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false},
      {pubkey: SYSVAR_STAKE_HISTORY_PUBKEY, isSigner: false, isWritable: false},
      {pubkey: StakeProgram.programId, isSigner: false, isWritable: false},
      {pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false},
    ];

    if (solWithdrawAuthority) {
      keys.push({
        pubkey: solWithdrawAuthority,
        isSigner: true,
        isWritable: false,
      });
    }

    return new TransactionInstruction({
      programId: STAKE_POOL_PROGRAM_ID,
      keys,
      data,
    });
  }

  /**
   * Decode a deposit stake pool instruction and retrieve the instruction params.
   */
  static decodeDepositStake(
    instruction: TransactionInstruction,
  ): DepositStakeParams {
    this.checkProgramId(instruction.programId);
    this.checkKeyLength(instruction.keys, 11);

    decodeData(STAKE_POOL_INSTRUCTION_LAYOUTS.DepositStake, instruction.data);

    return {
      stakePool: instruction.keys[0].pubkey,
      validatorList: instruction.keys[1].pubkey,
      depositAuthority: instruction.keys[2].pubkey,
      withdrawAuthority: instruction.keys[3].pubkey,
      depositStake: instruction.keys[4].pubkey,
      validatorStake: instruction.keys[5].pubkey,
      reserveStake: instruction.keys[6].pubkey,
      destinationPoolAccount: instruction.keys[7].pubkey,
      managerFeeAccount: instruction.keys[8].pubkey,
      referralPoolAccount: instruction.keys[9].pubkey,
      poolMint: instruction.keys[10].pubkey,
    };
  }

  /**
   * Decode a deposit sol instruction and retrieve the instruction params.
   */
  static decodeDepositSol(
    instruction: TransactionInstruction,
  ): DepositSolParams {
    this.checkProgramId(instruction.programId);
    this.checkKeyLength(instruction.keys, 9);

    const {amount} = decodeData(
      STAKE_POOL_INSTRUCTION_LAYOUTS.DepositSol,
      instruction.data,
    );

    return {
      stakePool: instruction.keys[0].pubkey,
      depositAuthority: instruction.keys[1].pubkey,
      withdrawAuthority: instruction.keys[2].pubkey,
      reserveStake: instruction.keys[3].pubkey,
      fundingAccount: instruction.keys[4].pubkey,
      destinationPoolAccount: instruction.keys[5].pubkey,
      managerFeeAccount: instruction.keys[6].pubkey,
      referralPoolAccount: instruction.keys[7].pubkey,
      poolMint: instruction.keys[8].pubkey,
      lamports: amount,
    };
  }

  /**
   * @internal
   */
  private static checkProgramId(programId: PublicKey) {
    if (!programId.equals(StakeProgram.programId)) {
      throw new Error('Invalid instruction; programId is not StakeProgram');
    }
  }

  /**
   * @internal
   */
  private static checkKeyLength(keys: Array<any>, expectedLength: number) {
    if (keys.length < expectedLength) {
      throw new Error(
        `Invalid instruction; found ${keys.length} keys, expected at least ${expectedLength}`,
      );
    }
  }
}
