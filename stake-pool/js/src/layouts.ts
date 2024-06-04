import { Layout, publicKey, u64, option, vec } from './codecs';
import { struct, Layout as LayoutCls, u8, u32 } from 'buffer-layout';
import { PublicKey } from '@solana/web3.js';
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

export class FutureEpochLayout<T> extends LayoutCls<T | null> {
  layout: Layout<T>;
  discriminator: Layout<number>;

  constructor(layout: Layout<T>, property?: string) {
    super(-1, property);
    this.layout = layout;
    this.discriminator = u8();
  }

  encode(src: T | null, b: Buffer, offset = 0): number {
    if (src === null || src === undefined) {
      return this.discriminator.encode(0, b, offset);
    }
    // This isn't right, but we don't typically encode outside of tests
    this.discriminator.encode(2, b, offset);
    return this.layout.encode(src, b, offset + 1) + 1;
  }

  decode(b: Buffer, offset = 0): T | null {
    const discriminator = this.discriminator.decode(b, offset);
    if (discriminator === 0) {
      return null;
    } else if (discriminator === 1 || discriminator === 2) {
      return this.layout.decode(b, offset + 1);
    }
    throw new Error('Invalid future epoch ' + this.property);
  }

  getSpan(b: Buffer, offset = 0): number {
    const discriminator = this.discriminator.decode(b, offset);
    if (discriminator === 0) {
      return 1;
    } else if (discriminator === 1 || discriminator === 2) {
      return this.layout.getSpan(b, offset + 1) + 1;
    }
    throw new Error('Invalid future epoch ' + this.property);
  }
}

export function futureEpoch<T>(layout: Layout<T>, property?: string): LayoutCls<T | null> {
  return new FutureEpochLayout<T>(layout, property);
}

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
export interface Lockup {
  unixTimestamp: BN;
  epoch: BN;
  custodian: PublicKey;
}

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
  futureEpoch(struct(feeFields), 'nextEpochFee'),
  option(publicKey(), 'preferredDepositValidatorVoteAddress'),
  option(publicKey(), 'preferredWithdrawValidatorVoteAddress'),
  struct(feeFields, 'stakeDepositFee'),
  struct(feeFields, 'stakeWithdrawalFee'),
  futureEpoch(struct(feeFields), 'nextStakeWithdrawalFee'),
  u8('stakeReferralFee'),
  option(publicKey(), 'solDepositAuthority'),
  struct(feeFields, 'solDepositFee'),
  u8('solReferralFee'),
  option(publicKey(), 'solWithdrawAuthority'),
  struct(feeFields, 'solWithdrawalFee'),
  futureEpoch(struct(feeFields), 'nextSolWithdrawalFee'),
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
