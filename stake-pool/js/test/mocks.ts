import { AccountInfo, LAMPORTS_PER_SOL, PublicKey, StakeProgram } from '@solana/web3.js';
import BN from 'bn.js';
import { ValidatorStakeInfo } from '../src';
import { AccountLayout, TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { ValidatorListLayout, ValidatorStakeInfoStatus } from '../src/layouts';

export const CONSTANTS = {
  poolTokenAccount: new PublicKey('GQkqTamwqjaNDfsbNm7r3aXPJ4oTSqKC3d5t2PF9Smqd'),
  validatorStakeAccountAddress: new PublicKey(
    new BN('69184b7f1bc836271c4ac0e29e53eb38a38ea0e7bcde693c45b30d1592a5a678', 'hex'),
  ),
};

export const stakePoolMock = {
  accountType: 1,
  manager: new PublicKey(11),
  staker: new PublicKey(12),
  stakeDepositAuthority: new PublicKey(13),
  stakeWithdrawBumpSeed: 255,
  validatorList: new PublicKey(14),
  reserveStake: new PublicKey(15),
  poolMint: new PublicKey(16),
  managerFeeAccount: new PublicKey(17),
  tokenProgramId: new PublicKey(18),
  totalLamports: new BN(LAMPORTS_PER_SOL * 999),
  poolTokenSupply: new BN(LAMPORTS_PER_SOL * 100),
  lastUpdateEpoch: new BN('7c', 'hex'),
  lockup: {
    unixTimestamp: new BN(Date.now()),
    epoch: new BN(1),
    custodian: new PublicKey(0),
  },
  epochFee: {
    denominator: new BN(0),
    numerator: new BN(0),
  },
  nextEpochFee: {
    denominator: new BN(0),
    numerator: new BN(0),
  },
  preferredDepositValidatorVoteAddress: new PublicKey(1),
  preferredWithdrawValidatorVoteAddress: new PublicKey(2),
  stakeDepositFee: {
    denominator: new BN(0),
    numerator: new BN(0),
  },
  stakeWithdrawalFee: {
    denominator: new BN(0),
    numerator: new BN(0),
  },
  nextStakeWithdrawalFee: {
    denominator: new BN(0),
    numerator: new BN(0),
  },
  stakeReferralFee: 0,
  solDepositAuthority: new PublicKey(0),
  solDepositFee: {
    denominator: new BN(0),
    numerator: new BN(0),
  },
  solReferralFee: 0,
  solWithdrawAuthority: new PublicKey(0),
  solWithdrawalFee: {
    denominator: new BN(0),
    numerator: new BN(0),
  },
  nextSolWithdrawalFee: {
    denominator: new BN(0),
    numerator: new BN(0),
  },
  lastEpochPoolTokenSupply: new BN(0),
  lastEpochTotalLamports: new BN(0),
};

export const validatorListMock = {
  accountType: 0,
  maxValidators: 100,
  validators: <ValidatorStakeInfo[]>[
    {
      status: ValidatorStakeInfoStatus.ReadyForRemoval,
      voteAccountAddress: new PublicKey(
        new BN('a9946a889af14fd3c9b33d5df309489d9699271a6b09ff3190fcb41cf21a2f8c', 'hex'),
      ),
      lastUpdateEpoch: new BN('c3', 'hex'),
      activeStakeLamports: new BN(123),
      transientStakeLamports: new BN(999),
      transientSeedSuffixStart: new BN(999),
      transientSeedSuffixEnd: new BN(999),
    },
    {
      status: ValidatorStakeInfoStatus.Active,
      voteAccountAddress: new PublicKey(
        new BN('3796d40645ee07e3c64117e3f73430471d4c40465f696ebc9b034c1fc06a9f7d', 'hex'),
      ),
      lastUpdateEpoch: new BN('c3', 'hex'),
      activeStakeLamports: new BN(LAMPORTS_PER_SOL * 100),
      transientStakeLamports: new BN(22),
      transientSeedSuffixStart: new BN(0),
      transientSeedSuffixEnd: new BN(0),
    },
    {
      status: ValidatorStakeInfoStatus.Active,
      voteAccountAddress: new PublicKey(
        new BN('e4e37d6f2e80c0bb0f3da8a06304e57be5cda6efa2825b86780aa320d9784cf8', 'hex'),
      ),
      lastUpdateEpoch: new BN('c3', 'hex'),
      activeStakeLamports: new BN(0),
      transientStakeLamports: new BN(0),
      transientSeedSuffixStart: new BN('a', 'hex'),
      transientSeedSuffixEnd: new BN('a', 'hex'),
    },
  ],
};

export function mockTokenAccount(amount = 0) {
  const data = Buffer.alloc(165);
  AccountLayout.encode(
    {
      mint: stakePoolMock.poolMint,
      owner: new PublicKey(0),
      amount: BigInt(amount),
      delegateOption: 0,
      delegate: new PublicKey(0),
      delegatedAmount: BigInt(0),
      state: 1,
      isNativeOption: 0,
      isNative: BigInt(0),
      closeAuthorityOption: 0,
      closeAuthority: new PublicKey(0),
    },
    data,
  );

  return <AccountInfo<any>>{
    executable: true,
    owner: TOKEN_PROGRAM_ID,
    lamports: amount,
    data,
  };
}

export const mockRpc = (data: any): any => {
  const value = {
    owner: StakeProgram.programId,
    lamports: LAMPORTS_PER_SOL,
    data: data,
    executable: false,
    rentEpoch: 0,
  };
  return {
    context: {
      slot: 11,
    },
    value: value,
  };
};

export const stakeAccountData = {
  program: 'stake',
  parsed: {
    type: 'delegated',
    info: {
      meta: {
        rentExemptReserve: new BN(1),
        lockup: {
          epoch: 32,
          unixTimestamp: 2,
          custodian: new PublicKey(12),
        },
        authorized: {
          staker: new PublicKey(12),
          withdrawer: new PublicKey(12),
        },
      },
      stake: {
        delegation: {
          voter: new PublicKey(
            new BN('e4e37d6f2e80c0bb0f3da8a06304e57be5cda6efa2825b86780aa320d9784cf8', 'hex'),
          ),
          stake: new BN(0),
          activationEpoch: new BN(1),
          deactivationEpoch: new BN(1),
          warmupCooldownRate: 1.2,
        },
        creditsObserved: 1,
      },
    },
  },
};

export const uninitializedStakeAccount = {
  program: 'stake',
  parsed: {
    type: 'uninitialized',
  },
};

export function mockValidatorsStakeAccount() {
  const data = Buffer.alloc(1024);
  return <AccountInfo<any>>{
    executable: false,
    owner: StakeProgram.programId,
    lamports: 3000000000,
    data,
  };
}

export function mockValidatorList() {
  const data = Buffer.alloc(1024);
  ValidatorListLayout.encode(validatorListMock, data);
  return <AccountInfo<any>>{
    executable: true,
    owner: new PublicKey(0),
    lamports: 0,
    data,
  };
}
