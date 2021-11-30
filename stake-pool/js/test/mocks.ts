import { AccountInfo, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import { ValidatorStakeInfo } from "../src";
import { ACCOUNT_LAYOUT, VALIDATOR_LIST_LAYOUT, ValidatorStakeInfoStatus } from "../src/layouts";

export const stakePoolMock = {
  accountType: 1,
  manager: new PublicKey(
    new BN(
      'dc23cda2ad09ddec126f89ed7f67d06a4d167cca996503f1a1b3b5a13625964f',
      'hex',
    ),
  ),
  staker: new PublicKey(
    new BN(
      'dc23cda2ad09ddec126f89ed7f67d06a4d167cca996503f1a1b3b5a13625964f',
      'hex',
    ),
  ),
  stakeDepositAuthority: new PublicKey(
    new BN(
      new Buffer(
        '5911e7451a1a854fdc9e495081790f293eba623f8ec7e2b9d34a5fd25c7009bb',
        'hex',
      ),
    ),
  ),
  stakeWithdrawBumpSeed: 255,
  validatorList: new PublicKey(
    new BN(
      '7103ba4895b8804263197364da9e791db96ec8f0c8ca184dd666e69013838610',
      'hex',
    ),
  ),
  reserveStake: new PublicKey(
    new BN(
      '74a5b1ab8442103baa8bd39ab8494eb034e96035ac664e1693bb3eef458761ee',
      'hex',
    ),
  ),
  poolMint: new PublicKey(
    new BN(
      '8722bf107b95d2620008d256b18c13fa3a46ab7f643c24cf7656f57267563e00',
      'hex',
    ),
  ),
  managerFeeAccount: new PublicKey(
    new BN(
      new Buffer(
        'b783b4dcd341cbca22e781bbd49b2d16908a844a21b98e26b69d44fc50e1db0f',
        'hex',
      ),
    ),
  ),
  tokenProgramId: new PublicKey(
    new BN(
      'a900ff7e85f58c3a91375b5fed85b41cac79ebce46e1cbd993a165d7e1f6dd06',
      'hex',
    ),
  ),
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
}

export function mockTokenAccount(amount = 0) {
  const data = Buffer.alloc(1024);
  ACCOUNT_LAYOUT.encode({
    state: 0,
    mint: stakePoolMock.poolMint,
    owner: new PublicKey(0),
    amount: new BN(amount),
  }, data)
  return <AccountInfo<any>>{
    executable: true,
    owner: new PublicKey(0),
    lamports: amount,
    data,
  }
}

export function mockValidatorList() {
  const data = Buffer.alloc(1024);
  VALIDATOR_LIST_LAYOUT.encode(validatorListMock, data)
  return <AccountInfo<any>>{
    executable: true,
    owner: new PublicKey(0),
    lamports: 0,
    data,
  }
}
