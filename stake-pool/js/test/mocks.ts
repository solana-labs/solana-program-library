import {AccountInfo, LAMPORTS_PER_SOL, PublicKey} from '@solana/web3.js';
import BN from 'bn.js';
import {ValidatorStakeInfo} from '../src';
import {
  ValidatorStakeInfoStatus,
  AccountLayout,
  ValidatorListLayout,
} from '../src/layouts';

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
  nextWithdrawalFee: {
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
        new BN(
          'a9946a889af14fd3c9b33d5df309489d9699271a6b09ff3190fcb41cf21a2f8c',
          'hex',
        ),
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
        new BN(
          '3796d40645ee07e3c64117e3f73430471d4c40465f696ebc9b034c1fc06a9f7d',
          'hex',
        ),
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
        new BN(
          'e4e37d6f2e80c0bb0f3da8a06304e57be5cda6efa2825b86780aa320d9784cf8',
          'hex',
        ),
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
  const data = Buffer.alloc(1024);
  AccountLayout.encode(
    {
      state: 0,
      mint: stakePoolMock.poolMint,
      owner: new PublicKey(0),
      amount: new BN(amount),
      // address: new PublicKey(0),
      // delegate: null,
      // delegatedAmount: new BN(0),
      // isInitialized: true,
      // isFrozen: false,
      // isNative: false,
      // rentExemptReserve: null,
      // closeAuthority: null,
    },
    data,
  );

  return <AccountInfo<any>>{
    executable: true,
    owner: new PublicKey(0),
    lamports: amount,
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
