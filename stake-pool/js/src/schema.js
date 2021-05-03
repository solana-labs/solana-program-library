import borsh from 'borsh';
// Class wrapping a plain object
export class Assignable {
    constructor(properties) {
        Object.keys(properties).forEach((key) => {
            this[key] = properties[key];
        });
    }
    encode() {
        return Buffer.from(borsh.serialize(SCHEMA, this));
    }
    static decode(data) {
        return borsh.deserializeUnchecked(SCHEMA, this, data);
    }
}
// Class representing a Rust-compatible enum, since enums are only strings or
// numbers in pure JS
export class Enum extends Assignable {
    constructor(properties) {
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
}
export class AccountType extends Enum {
}
export class AccountTypeEnum extends Assignable {
}
export class StakePool extends Assignable {
}
export class ValidatorList extends Assignable {
}
export class ValidatorStakeInfo extends Assignable {
}
export class StakeStatus extends Enum {
}
export class StakeStatusEnum extends Assignable {
}
export class PublicKey extends Assignable {
}
export const SCHEMA = constructStakePoolSchema();
export function constructStakePoolSchema() {
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
    SCHEMA.set(AccountTypeEnum, { kind: 'struct', fields: [] });
    SCHEMA.set(StakePool, {
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
    SCHEMA.set(ValidatorList, {
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
    SCHEMA.set(StakeStatusEnum, { kind: 'struct', fields: [] });
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
