/**
 * Based on https://github.com/solana-labs/solana-web3.js/blob/master/src/stake-program.ts
 */
import { encodeData, decodeData, InstructionType } from './copied-from-solana-web3/instruction';
import {
  PublicKey,
  Transaction,
  TransactionInstruction,
  StakeProgram,
  SystemProgram,
  StakeAuthorizationLayout,
  SYSVAR_CLOCK_PUBKEY,
  SYSVAR_STAKE_HISTORY_PUBKEY,
} from '@solana/web3.js';
import * as BufferLayout from '@solana/buffer-layout';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { solToLamports } from './utils';

export const MIN_STAKE_BALANCE = solToLamports(1.0);
export const STAKE_STATE_LEN = 200;

/// Minimum amount of staked SOL required in a validator stake account to allow
/// for merges without a mismatch on credits observed
// export const MINIMUM_ACTIVE_STAKE = LAMPORTS_PER_SOL / 1_000;

/**
 * An enumeration of valid StakePoolInstructionType's
 */
export type StakePoolInstructionType =
  | 'Initialize'
  | 'Deposit'
  | 'DepositSol'
  | 'WithdrawStake'
  | 'WithdrawSol'
  | 'SetFundingAuthority';

/**
 * Defines which deposit authority to update in the `SetDepositAuthority`
 */
export enum DepositType {
  /// Sets the stake deposit authority
  Stake,
  /// Sets the SOL deposit authority
  Sol,
}

/**
 * An enumeration of valid stake InstructionType's
 * @internal
 */
export const STAKE_POOL_INSTRUCTION_LAYOUTS: {
  [type in StakePoolInstructionType]: InstructionType;
} = Object.freeze({
  Initialize: {
    index: 0,
    layout: BufferLayout.struct([
      BufferLayout.u8('instruction'),
      BufferLayout.ns64('fee_denominator'),
      BufferLayout.ns64('fee_numerator'),
      BufferLayout.ns64('withdrawal_fee_denominator'),
      BufferLayout.ns64('withdrawal_fee_numerator'),
      BufferLayout.u32('max_validators'),
    ]),
  },
  Deposit: {
    index: 9,
    layout: BufferLayout.struct([BufferLayout.u8('instruction')]),
  },
  ///   Withdraw the token from the pool at the current ratio.
  ///
  ///   Succeeds if the stake account has enough SOL to cover the desired amount
  ///   of pool tokens, and if the withdrawal keeps the total staked amount
  ///   above the minimum of rent-exempt amount + 0.001 SOL.
  ///
  ///   When allowing withdrawals, the order of priority goes:
  ///
  ///   * preferred withdraw validator stake account (if set)
  ///   * validator stake accounts
  ///   * transient stake accounts
  ///   * reserve stake account
  ///
  ///   A user can freely withdraw from a validator stake account, and if they
  ///   are all at the minimum, then they can withdraw from transient stake
  ///   accounts, and if they are all at minimum, then they can withdraw from
  ///   the reserve.
  ///
  ///   0. `[w]` Stake pool
  ///   1. `[w]` Validator stake list storage account
  ///   2. `[]` Stake pool withdraw authority
  ///   3. `[w]` Validator or reserve stake account to split
  ///   4. `[w]` Unitialized stake account to receive withdrawal
  ///   5. `[]` User account to set as a new withdraw authority
  ///   6. `[s]` User transfer authority, for pool token account
  ///   7. `[w]` User account with pool tokens to burn from
  ///   8. `[w]` Account to receive pool fee tokens
  ///   9. `[w]` Pool token mint account
  ///  10. `[]` Sysvar clock account (required)
  ///  11. `[]` Pool token program id
  ///  12. `[]` Stake program id,
  ///  userdata: amount of pool tokens to withdraw
  WithdrawStake: {
    index: 10,
    layout: BufferLayout.struct([BufferLayout.u8('instruction'), BufferLayout.ns64('lamports')]),
  },
  ///   Deposit SOL directly into the pool's reserve account. The output is a "pool" token
  ///   representing ownership into the pool. Inputs are converted to the current ratio.
  ///
  ///   0. `[w]` Stake pool
  ///   1. `[]` Stake pool withdraw authority
  ///   2. `[w]` Reserve stake account, to deposit SOL
  ///   3. `[s]` Account providing the lamports to be deposited into the pool
  ///   4. `[w]` User account to receive pool tokens
  ///   5. `[w]` Account to receive fee tokens
  ///   6. `[w]` Account to receive a portion of fee as referral fees
  ///   7. `[w]` Pool token mint account
  ///   8. `[]` System program account
  ///   9. `[]` Token program id
  ///  10. `[s]` (Optional) Stake pool sol deposit authority.
  DepositSol: {
    index: 14,
    layout: BufferLayout.struct([BufferLayout.u8('instruction'), BufferLayout.ns64('lamports')]),
  },
  ///  (Manager only) Update SOL deposit authority
  ///
  ///  0. `[w]` StakePool
  ///  1. `[s]` Manager
  ///  2. '[]` New authority pubkey or none
  SetFundingAuthority: {
    index: 15,
    layout: BufferLayout.struct([BufferLayout.u8('instruction'), BufferLayout.u32('fundingType')]),
  },
  ///   Withdraw SOL directly from the pool's reserve account. Fails if the
  ///   reserve does not have enough SOL.
  ///
  ///   0. `[w]` Stake pool
  ///   1. `[]` Stake pool withdraw authority
  ///   2. `[s]` User transfer authority, for pool token account
  ///   3. `[w]` User account to burn pool tokens
  ///   4. `[w]` Reserve stake account, to withdraw SOL
  ///   5. `[w]` Account receiving the lamports from the reserve, must be a system account
  ///   6. `[w]` Account to receive pool fee tokens
  ///   7. `[w]` Pool token mint account
  ///   8. '[]' Clock sysvar
  ///   9. '[]' Stake history sysvar
  ///  10. `[]` Stake program account
  ///  11. `[]` Token program id
  ///  12. `[s]` (Optional) Stake pool sol withdraw authority
  WithdrawSol: {
    index: 16,
    layout: BufferLayout.struct([BufferLayout.u8('instruction'), BufferLayout.ns64('poolTokens')]),
  },
});

/**
 * Initialize stake instruction params
 */
export type InitializeStakePoolParams = {
  feeDenominator: number;
  feeNumerator: number;
  withdrawalDenominator: number;
  withdrawalNumerator: number;
  maxValidators: number;
};

/**
 * Deposit stake pool instruction params
 */
export type DepositStakePoolParams = {
  stakePoolPubkey: PublicKey;
  validatorListStorage: PublicKey;
  stakePoolDepositAuthority: PublicKey;
  stakePoolWithdrawAuthority: PublicKey;
  depositStakeAddress: PublicKey;
  depositStakeWithdrawAuthority: PublicKey;
  validatorStakeAccount: PublicKey;
  reserveStakeAccount: PublicKey;
  poolTokensTo: PublicKey;
  poolMint: PublicKey;
};

/**
 * Withdraw stake pool instruction params
 */
export type WithdrawStakePoolParams = {
  stakePoolPubkey: PublicKey;
  validatorListStorage: PublicKey;
  stakePoolWithdrawAuthority: PublicKey;
  stakeToSplit: PublicKey;
  stakeToReceive: PublicKey;
  userStakeAuthority: PublicKey;
  userTransferAuthority: PublicKey;
  userPoolTokenAccount: PublicKey;
  managerFeeAccount: PublicKey;
  poolMint: PublicKey;
  lamports: number;
};

/**
 * Withdraw sol instruction params
 */
export type WithdrawSolParams = {
  stakePoolPubkey: PublicKey;
  solWithdrawAuthority: PublicKey | undefined;
  stakePoolWithdrawAuthority: PublicKey;
  userTransferAuthority: PublicKey;
  poolTokensFrom: PublicKey;
  reserveStakeAccount: PublicKey;
  lamportsTo: PublicKey;
  managerFeeAccount: PublicKey;
  poolMint: PublicKey;
  poolTokens: number;
};

/**
 * Deposit sol instruction params
 */
export type DepositSolParams = {
  stakePoolPubkey: PublicKey;
  depositAuthority?: PublicKey;
  withdrawAuthority: PublicKey;
  reserveStakeAccount: PublicKey;
  lamportsFrom: PublicKey;
  poolTokensTo: PublicKey;
  managerFeeAccount: PublicKey;
  referrerPoolTokensAccount: PublicKey;
  poolMint: PublicKey;
  lamports: number;
};

export interface WithdrawAccount {
  stakeAddress: PublicKey;
  voteAddress?: PublicKey;
  poolAmount: number;
}

/**
 * Stake Pool Instruction class
 */
export class StakePoolInstruction {
  /**
   * Decode a initialize stake pool instruction and retrieve the instruction params.
   */
  static decodeInitialize(instruction: TransactionInstruction): InitializeStakePoolParams {
    this.checkProgramId(instruction.programId);
    this.checkKeyLength(instruction.keys, 6);
    const {
      feeDenominator,
      feeNumerator,
      withdrawalDenominator,
      withdrawalNumerator,
      maxValidators,
    } = decodeData(STAKE_POOL_INSTRUCTION_LAYOUTS.Initialize, instruction.data);

    return {
      feeDenominator: feeDenominator,
      feeNumerator: feeNumerator,
      withdrawalDenominator: withdrawalDenominator,
      withdrawalNumerator: withdrawalNumerator,
      maxValidators: maxValidators,
    };
  }

  /**
   * Decode a deposit stake pool instruction and retrieve the instruction params.
   */
  static decodeDeposit(instruction: TransactionInstruction): DepositStakePoolParams {
    this.checkProgramId(instruction.programId);
    this.checkKeyLength(instruction.keys, 6);
    decodeData(STAKE_POOL_INSTRUCTION_LAYOUTS.Deposit, instruction.data);

    return {
      stakePoolPubkey: instruction.keys[0].pubkey,
      validatorListStorage: instruction.keys[1].pubkey,
      stakePoolDepositAuthority: instruction.keys[2].pubkey,
      stakePoolWithdrawAuthority: instruction.keys[3].pubkey,
      depositStakeAddress: instruction.keys[4].pubkey,
      depositStakeWithdrawAuthority: instruction.keys[5].pubkey,
      validatorStakeAccount: instruction.keys[6].pubkey,
      reserveStakeAccount: instruction.keys[7].pubkey,
      poolTokensTo: instruction.keys[8].pubkey,
      poolMint: instruction.keys[9].pubkey,
    };
  }

  /**
   * Decode a deposit sol instruction and retrieve the instruction params.
   */
  static decodeDepositSol(instruction: TransactionInstruction): DepositSolParams {
    this.checkProgramId(instruction.programId);
    this.checkKeyLength(instruction.keys, 9);

    const { amount } = decodeData(STAKE_POOL_INSTRUCTION_LAYOUTS.DepositSol, instruction.data);

    return {
      stakePoolPubkey: instruction.keys[0].pubkey,
      depositAuthority: instruction.keys[1].pubkey,
      withdrawAuthority: instruction.keys[2].pubkey,
      reserveStakeAccount: instruction.keys[3].pubkey,
      lamportsFrom: instruction.keys[4].pubkey,
      poolTokensTo: instruction.keys[5].pubkey,
      managerFeeAccount: instruction.keys[6].pubkey,
      referrerPoolTokensAccount: instruction.keys[7].pubkey,
      poolMint: instruction.keys[8].pubkey,
      lamports: amount,
    };
  }

  /**
   * @internal
   */
  static checkProgramId(programId: PublicKey) {
    if (!programId.equals(StakeProgram.programId)) {
      throw new Error('invalid instruction; programId is not StakeProgram');
    }
  }

  /**
   * @internal
   */
  static checkKeyLength(keys: Array<any>, expectedLength: number) {
    if (keys.length < expectedLength) {
      throw new Error(
        `invalid instruction; found ${keys.length} keys, expected at least ${expectedLength}`,
      );
    }
  }
}

/**
 * Factory class for transactions to interact with the Stake program
 */
export class StakePoolProgram {
  /**
   * Public key that identifies the Stake Pool program
   */
  static programId: PublicKey = new PublicKey('SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy');

  static tokenProgramId: PublicKey = TOKEN_PROGRAM_ID;

  static stakeProgramId = StakeProgram.programId;

  static initialize(params: InitializeStakePoolParams): Transaction {
    const {
      feeDenominator,
      feeNumerator,
      withdrawalDenominator,
      withdrawalNumerator,
      maxValidators,
    } = params;

    const type = STAKE_POOL_INSTRUCTION_LAYOUTS.Initialize;

    const data = encodeData(type, {
      feeDenominator,
      feeNumerator,
      withdrawalDenominator,
      withdrawalNumerator,
      maxValidators,
    });

    console.log(data);

    return new Transaction().add();
  }

  static deposit(params: DepositStakePoolParams): Transaction {
    const {
      stakePoolPubkey,
      validatorListStorage,
      stakePoolDepositAuthority,
      stakePoolWithdrawAuthority,
      depositStakeAddress,
      depositStakeWithdrawAuthority,
      validatorStakeAccount,
      reserveStakeAccount,
      poolTokensTo,
      poolMint,
    } = params;

    const type = STAKE_POOL_INSTRUCTION_LAYOUTS.Deposit;
    const data = encodeData(type);

    return new Transaction().add(
      StakeProgram.authorize({
        stakePubkey: depositStakeAddress,
        authorizedPubkey: depositStakeWithdrawAuthority,
        newAuthorizedPubkey: stakePoolDepositAuthority,
        stakeAuthorizationType: StakeAuthorizationLayout.Staker,
      }),
      StakeProgram.authorize({
        stakePubkey: depositStakeAddress,
        authorizedPubkey: depositStakeWithdrawAuthority,
        newAuthorizedPubkey: stakePoolDepositAuthority,
        stakeAuthorizationType: StakeAuthorizationLayout.Withdrawer,
      }),
      {
        keys: [
          { pubkey: stakePoolPubkey, isSigner: false, isWritable: true },
          { pubkey: validatorListStorage, isSigner: false, isWritable: true },
          { pubkey: stakePoolDepositAuthority, isSigner: false, isWritable: false },
          { pubkey: stakePoolWithdrawAuthority, isSigner: false, isWritable: false },
          { pubkey: depositStakeAddress, isSigner: false, isWritable: true },
          { pubkey: validatorStakeAccount, isSigner: false, isWritable: true },
          { pubkey: reserveStakeAccount, isSigner: false, isWritable: true },
          { pubkey: poolTokensTo, isSigner: false, isWritable: true },
          { pubkey: poolMint, isSigner: false, isWritable: true },
          { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
          { pubkey: SYSVAR_STAKE_HISTORY_PUBKEY, isSigner: false, isWritable: false },
          { pubkey: this.tokenProgramId, isSigner: false, isWritable: false },
          { pubkey: this.stakeProgramId, isSigner: false, isWritable: false },
        ],
        programId: this.programId,
        data,
      },
    );
  }

  static depositSolInstruction(params: DepositSolParams): TransactionInstruction {
    const {
      stakePoolPubkey,
      depositAuthority,
      withdrawAuthority,
      reserveStakeAccount,
      lamportsFrom,
      poolTokensTo,
      managerFeeAccount,
      referrerPoolTokensAccount,
      poolMint,
      lamports,
    } = params;

    const type = STAKE_POOL_INSTRUCTION_LAYOUTS.DepositSol;
    const data = encodeData(type, { lamports });

    const keys = [
      { pubkey: stakePoolPubkey, isSigner: false, isWritable: true },
      { pubkey: withdrawAuthority, isSigner: false, isWritable: false },
      { pubkey: reserveStakeAccount, isSigner: false, isWritable: true },
      { pubkey: lamportsFrom, isSigner: true, isWritable: false },
      { pubkey: poolTokensTo, isSigner: false, isWritable: true },
      { pubkey: managerFeeAccount, isSigner: false, isWritable: true },
      { pubkey: referrerPoolTokensAccount, isSigner: false, isWritable: true },
      { pubkey: poolMint, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      { pubkey: this.tokenProgramId, isSigner: false, isWritable: false },
    ];

    if (depositAuthority) {
      keys.push({
        pubkey: depositAuthority,
        isSigner: false,
        isWritable: false,
      });
    }

    return new TransactionInstruction({
      programId: this.programId,
      keys,
      data,
    });
  }

  static withdrawStakeInstruction(params: WithdrawStakePoolParams) {
    const {
      stakePoolPubkey,
      validatorListStorage,
      stakePoolWithdrawAuthority,
      stakeToSplit,
      stakeToReceive,
      userStakeAuthority,
      userTransferAuthority,
      userPoolTokenAccount,
      managerFeeAccount,
      poolMint,
      lamports,
    } = params;

    const type = STAKE_POOL_INSTRUCTION_LAYOUTS.WithdrawStake;
    const data = encodeData(type, { lamports });

    const keys = [
      { pubkey: stakePoolPubkey, isSigner: false, isWritable: true },
      { pubkey: validatorListStorage, isSigner: false, isWritable: true },
      { pubkey: stakePoolWithdrawAuthority, isSigner: false, isWritable: false },
      { pubkey: stakeToSplit, isSigner: false, isWritable: true },
      { pubkey: stakeToReceive, isSigner: false, isWritable: true },
      { pubkey: userStakeAuthority, isSigner: false, isWritable: false },
      { pubkey: userTransferAuthority, isSigner: true, isWritable: false },
      { pubkey: userPoolTokenAccount, isSigner: false, isWritable: true },
      { pubkey: managerFeeAccount, isSigner: false, isWritable: true },
      { pubkey: poolMint, isSigner: false, isWritable: true },
      { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: this.tokenProgramId, isSigner: false, isWritable: false },
      { pubkey: this.stakeProgramId, isSigner: false, isWritable: false },
    ];

    return new TransactionInstruction({
      programId: this.programId,
      keys,
      data,
    });
  }

  static withdrawSolInstruction(params: WithdrawSolParams) {
    const {
      stakePoolPubkey,
      solWithdrawAuthority,
      stakePoolWithdrawAuthority,
      userTransferAuthority,
      poolTokensFrom,
      reserveStakeAccount,
      lamportsTo,
      managerFeeAccount,
      poolMint,
      poolTokens,
    } = params;

    const type = STAKE_POOL_INSTRUCTION_LAYOUTS.WithdrawSol;
    const data = encodeData(type, { poolTokens });

    const keys = [
      { pubkey: stakePoolPubkey, isSigner: false, isWritable: true },
      { pubkey: stakePoolWithdrawAuthority, isSigner: false, isWritable: false },
      { pubkey: userTransferAuthority, isSigner: true, isWritable: false },
      { pubkey: poolTokensFrom, isSigner: false, isWritable: true },
      { pubkey: reserveStakeAccount, isSigner: false, isWritable: true },
      { pubkey: lamportsTo, isSigner: false, isWritable: true },
      { pubkey: managerFeeAccount, isSigner: false, isWritable: true },
      { pubkey: poolMint, isSigner: false, isWritable: true },
      { pubkey: SYSVAR_CLOCK_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: SYSVAR_STAKE_HISTORY_PUBKEY, isSigner: false, isWritable: false },
      { pubkey: this.stakeProgramId, isSigner: false, isWritable: false },
      { pubkey: this.tokenProgramId, isSigner: false, isWritable: false },
    ];

    if (solWithdrawAuthority) {
      keys.push({
        pubkey: solWithdrawAuthority,
        isSigner: true,
        isWritable: false,
      });
    }

    return new TransactionInstruction({
      programId: this.programId,
      keys,
      data,
    });
  }
}
