import borsh from 'borsh';
import BN from 'bn.js';

// Class wrapping a plain object
export abstract class Assignable {
  constructor(properties: {[key: string]: any}) {
    Object.keys(properties).forEach((key: string) => {
      this[key] = properties[key];
    });
  }

  encode(): Buffer {
    return Buffer.from(borsh.serialize(SCHEMA, this));
  }

  static decode<T extends Assignable>(data: Buffer): T {
    return borsh.deserializeUnchecked(SCHEMA, this, data);
  }
}

// Class representing a Rust-compatible enum, since enums are only strings or
// numbers in pure JS
export abstract class Enum extends Assignable {
  enum: string;
  constructor(properties: any) {
    super(properties);
    if (Object.keys(properties).length !== 1) {
      throw new Error('Enum can only take single value');
    }
    this.enum = '';
    Object.keys(properties).forEach(key => {
      this.enum = key;
    });
  }
}

export class Fee extends Assignable {
  denominator: number;
  numerator: number;
}

export class AccountType extends Enum {}
export class AccountTypeEnum extends Assignable {}

export class StakePoolAccount extends Assignable {
  accountType: AccountType;
  manager: PublicKey;
  staker: PublicKey;
  depositAuthority: PublicKey;
  withdrawBumpSeed: number; // what is this? u8 in Rust
  validatorList: PublicKey;
  reserveStake: PublicKey;
  poolMint: PublicKey;
  managerFeeAccount: PublicKey;
  totalStakeLamports: BN;
  poolTokenSupply: BN;
  lastUpdateEpoch: BN;
}

export class ValidatorListAccount extends Assignable {
  accountType: AccountType;
  maxValidators: number;
  validators: [ValidatorStakeInfo];
}
export class ValidatorStakeInfo extends Assignable {
  status: StakeStatus;
  voteAccountAddress: PublicKey;
  stakeLamports: BN;
  lastUpdateEpoch: BN;
}
export class StakeStatus extends Enum {}
export class StakeStatusEnum extends Assignable {}

export class PublicKey extends Assignable {
  value: BN;
}

export const SCHEMA: borsh.Schema = constructStakePoolSchema();

/**
 * Borsh requires something called a Schema,
 * which is a Map (key-value pairs) that tell borsh how to deserialise the raw data
 * This function creates, populates and returns such a schema
 */
export function constructStakePoolSchema(): borsh.Schema {
  const SCHEMA = new Map();

  SCHEMA.set(PublicKey, {
    kind: 'struct',
    fields: [['value', 'u256']],
  });

  SCHEMA.set(Fee, {
    kind: 'struct',
    fields: [
      ['denominator', 'u64'],
      ['numerator', 'u64'],
    ],
  });

  SCHEMA.set(AccountType, {
    kind: 'enum',
    field: 'enum',
    values: [
      // if the account has not been initialized, the enum will be 0
      // FIXME
      ['Uninitialized', AccountTypeEnum],
      ['StakePool', AccountTypeEnum],
      ['ValidatorList', AccountTypeEnum],
    ],
  });

  SCHEMA.set(AccountTypeEnum, {kind: 'struct', fields: []});

  SCHEMA.set(StakePoolAccount, {
    kind: 'struct',
    fields: [
      ['accountType', AccountType],
      ['manager', PublicKey],
      ['staker', PublicKey],
      ['depositAuthority', PublicKey],
      ['withdrawBumpSeed', 'u8'],
      ['validatorList', PublicKey],
      ['reserveStake', PublicKey],
      ['poolMint', PublicKey],
      ['managerFeeAccount', PublicKey],
      ['tokenProgramId', PublicKey],
      ['totalStakeLamports', 'u64'],
      ['poolTokenSupply', 'u64'],
      ['lastUpdateEpoch', 'u64'],
      ['fee', Fee],
    ],
  });

  SCHEMA.set(ValidatorListAccount, {
    kind: 'struct',
    fields: [
      ['accountType', AccountType],
      ['maxValidators', 'u32'],
      ['validators', [ValidatorStakeInfo]],
    ],
  });

  SCHEMA.set(StakeStatus, {
    kind: 'enum',
    field: 'enum',
    values: [
      ['Active', StakeStatusEnum],
      ['DeactivatingTransient', StakeStatusEnum],
      ['ReadyForRemoval', StakeStatusEnum],
    ],
  });

  SCHEMA.set(StakeStatusEnum, {kind: 'struct', fields: []});

  SCHEMA.set(ValidatorStakeInfo, {
    kind: 'struct',
    fields: [
      ['status', StakeStatus],
      ['voteAccountAddress', PublicKey],
      ['stakeLamports', 'u64'],
      ['lastUpdateEpoch', 'u64'],
    ],
  });

  return SCHEMA;
}
