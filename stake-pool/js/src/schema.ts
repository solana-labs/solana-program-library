// import { serialize, deserializeUnchecked, BinaryReader, Schema, BorshError } from "borsh"
import borsh from "borsh"
import BN from 'bn.js';

export const SCHEMA = new Map();

// Class wrapping a plain object
export abstract class Assignable {
    constructor(properties: { [key: string]: any }) {
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

/* All stubs for now */
export class AccountType extends Enum { }
export class AccountTypeEnum extends Assignable { }
export class StakePool extends Assignable { }
export class ValidatorList extends Assignable { }
export class ValidatorStakeInfo extends Assignable { }
export class StakeStatus extends Enum { }
export class StakeStatusEnum extends Assignable { }

export class PublicKey extends Assignable {
    "value": BN;
}

export function constructStakePoolSchema() {
    const SCHEMA = new Map()

    SCHEMA.set(PublicKey, {
        kind: 'struct',
        fields: [
            ['value', 'u256'],
        ],
    })

    SCHEMA.set(Fee, {
        kind: 'struct',
        fields: [
            ['denominator', 'u64'],
            ['numerator', 'u64'],
        ],
    })

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
    })

    SCHEMA.set(AccountTypeEnum, { kind: 'struct', fields: [], })

    SCHEMA.set(StakePool, {
        kind: 'struct',
        fields: [
            ['account_type', AccountType],
            ['manager', PublicKey],
            ['staker', PublicKey],
            ['deposit_authority', PublicKey],
            ['withdraw_bump_seed', 'u8'],
            ['validator_list', PublicKey],
            ['reserve_stake', PublicKey],
            ['pool_mint', PublicKey],
            ['manager_fee_account', PublicKey],
            ['token_program_id', PublicKey],
            ['total_stake_lamports', 'u64'],
            ['pool_token_supply', 'u64'],
            ['last_update_epoch', 'u64'],
            ['fee', Fee],
        ],
    })

    SCHEMA.set(ValidatorList, {
        kind: 'struct',
        fields: [
            ['account_type', AccountType],
            ['max_validators', 'u32'],
            ['validators', [ValidatorStakeInfo]]
        ],
    })

    SCHEMA.set(StakeStatus, {
        kind: 'enum',
        field: 'enum',
        values: [
            ['Active', StakeStatusEnum],
            ['DeactivatingTransient', StakeStatusEnum],
            ['ReadyForRemoval', StakeStatusEnum],
        ],
    })

    SCHEMA.set(StakeStatusEnum, { kind: 'struct', fields: [] })

    SCHEMA.set(ValidatorStakeInfo, {
        kind: 'struct',
        fields: [
            ['status', StakeStatus],
            ['vote_account_address', PublicKey],
            ['stake_lamports', 'u64'],
            ['last_update_epoch', 'u64'],
        ],
    })

    return SCHEMA
}