import { publicKey, struct, u32, u64, u8, option, vec } from '@coral-xyz/borsh';
import { Lockup, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import {
  Infer,
  number,
  nullable,
  enums,
  type,
  coerce,
  instance,
  string,
  optional,
} from 'superstruct';

export interface Fee {
  denominator: BN;
  numerator: BN;
}

const feeFields = [u64('denominator'), u64('numerator')];

export enum AccountType {
  Uninitialized,
  StakePool,
  ValidatorList,
}

export const BigNumFromString = coerce(instance(BN), string(), (value) => {
  if (typeof value === 'string') return new BN(value, 10);
  throw new Error('invalid big num');
});

export const PublicKeyFromString = coerce(
  instance(PublicKey),
  string(),
  (value) => new PublicKey(value),
);

export type StakeAccountType = Infer<typeof StakeAccountType>;
export const StakeAccountType = enums(['uninitialized', 'initialized', 'delegated', 'rewardsPool']);

export type StakeMeta = Infer<typeof StakeMeta>;
export const StakeMeta = type({
  rentExemptReserve: BigNumFromString,
  authorized: type({
    staker: PublicKeyFromString,
    withdrawer: PublicKeyFromString,
  }),
  lockup: type({
    unixTimestamp: number(),
    epoch: number(),
    custodian: PublicKeyFromString,
  }),
});

export type StakeAccountInfo = Infer<typeof StakeAccountInfo>;
export const StakeAccountInfo = type({
  meta: StakeMeta,
  stake: nullable(
    type({
      delegation: type({
        voter: PublicKeyFromString,
        stake: BigNumFromString,
        activationEpoch: BigNumFromString,
        deactivationEpoch: BigNumFromString,
        warmupCooldownRate: number(),
      }),
      creditsObserved: number(),
    }),
  ),
});

export type StakeAccount = Infer<typeof StakeAccount>;
export const StakeAccount = type({
  type: StakeAccountType,
  info: optional(StakeAccountInfo),
});

export interface StakePool {
  accountType: AccountType;
  manager: PublicKey;
  staker: PublicKey;
  stakeDepositAuthority: PublicKey;
  stakeWithdrawBumpSeed: number;
  validatorList: PublicKey;
  reserveStake: PublicKey;
  poolMint: PublicKey;
  managerFeeAccount: PublicKey;
  tokenProgramId: PublicKey;
  totalLamports: BN;
  poolTokenSupply: BN;
  lastUpdateEpoch: BN;
  lockup: Lockup;
  epochFee: Fee;
  nextEpochFee?: Fee | undefined;
  preferredDepositValidatorVoteAddress?: PublicKey | undefined;
  preferredWithdrawValidatorVoteAddress?: PublicKey | undefined;
  stakeDepositFee: Fee;
  stakeWithdrawalFee: Fee;
  nextStakeWithdrawalFee?: Fee | undefined;
  stakeReferralFee: number;
  solDepositAuthority?: PublicKey | undefined;
  solDepositFee: Fee;
  solReferralFee: number;
  solWithdrawAuthority?: PublicKey | undefined;
  solWithdrawalFee: Fee;
  nextSolWithdrawalFee?: Fee | undefined;
  lastEpochPoolTokenSupply: BN;
  lastEpochTotalLamports: BN;
}

export const StakePoolLayout = struct<StakePool>([
  u8('accountType'),
  publicKey('manager'),
  publicKey('staker'),
  publicKey('stakeDepositAuthority'),
  u8('stakeWithdrawBumpSeed'),
  publicKey('validatorList'),
  publicKey('reserveStake'),
  publicKey('poolMint'),
  publicKey('managerFeeAccount'),
  publicKey('tokenProgramId'),
  u64('totalLamports'),
  u64('poolTokenSupply'),
  u64('lastUpdateEpoch'),
  struct([u64('unixTimestamp'), u64('epoch'), publicKey('custodian')], 'lockup'),
  struct(feeFields, 'epochFee'),
  option(struct(feeFields), 'nextEpochFee'),
  option(publicKey(), 'preferredDepositValidatorVoteAddress'),
  option(publicKey(), 'preferredWithdrawValidatorVoteAddress'),
  struct(feeFields, 'stakeDepositFee'),
  struct(feeFields, 'stakeWithdrawalFee'),
  option(struct(feeFields), 'nextStakeWithdrawalFee'),
  u8('stakeReferralFee'),
  option(publicKey(), 'solDepositAuthority'),
  struct(feeFields, 'solDepositFee'),
  u8('solReferralFee'),
  option(publicKey(), 'solWithdrawAuthority'),
  struct(feeFields, 'solWithdrawalFee'),
  option(struct(feeFields), 'nextSolWithdrawalFee'),
  u64('lastEpochPoolTokenSupply'),
  u64('lastEpochTotalLamports'),
]);

export enum ValidatorStakeInfoStatus {
  Active,
  DeactivatingTransient,
  ReadyForRemoval,
}

export interface ValidatorStakeInfo {
  status: ValidatorStakeInfoStatus;
  voteAccountAddress: PublicKey;
  activeStakeLamports: BN;
  transientStakeLamports: BN;
  transientSeedSuffixStart: BN;
  transientSeedSuffixEnd: BN;
  lastUpdateEpoch: BN;
}

export const ValidatorStakeInfoLayout = struct<ValidatorStakeInfo>([
  /// Amount of active stake delegated to this validator
  /// Note that if `last_update_epoch` does not match the current epoch then
  /// this field may not be accurate
  u64('activeStakeLamports'),
  /// Amount of transient stake delegated to this validator
  /// Note that if `last_update_epoch` does not match the current epoch then
  /// this field may not be accurate
  u64('transientStakeLamports'),
  /// Last epoch the active and transient stake lamports fields were updated
  u64('lastUpdateEpoch'),
  /// Start of the validator transient account seed suffixes
  u64('transientSeedSuffixStart'),
  /// End of the validator transient account seed suffixes
  u64('transientSeedSuffixEnd'),
  /// Status of the validator stake account
  u8('status'),
  /// Validator vote account address
  publicKey('voteAccountAddress'),
]);

export interface ValidatorList {
  /// Account type, must be ValidatorList currently
  accountType: number;
  /// Maximum allowable number of validators
  maxValidators: number;
  /// List of stake info for each validator in the pool
  validators: ValidatorStakeInfo[];
}

export const ValidatorListLayout = struct<ValidatorList>([
  u8('accountType'),
  u32('maxValidators'),
  vec(ValidatorStakeInfoLayout, 'validators'),
]);
