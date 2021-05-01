export default function constructStakePoolSchema() {
    const SCHEMA = new Map()
    SCHEMA.set('Fee', {
        kind: 'struct',
        fields: [
            ['denominator', 'u64'],
            ['numerator', 'u64'],
        ],
    })

    SCHEMA.set('AccountType', {
        kind: 'enum',
        field: 'enum',
        values: [
            // if the account has not been initialized, the enum will be 0
            ['Uninitialized', 0],
            ['StakePool', StakePoolEnum],
            ['ValidatorList', ValidatorListEnum],
        ],
    })

    SCHEMA.set('StakePoolEnum', { kind: 'struct', fields: [], })
    SCHEMA.set('ValidatorListEnum', { kind: 'struct', fields: [], })

    SCHEMA.set('StakePool', {
        kind: 'struct',
        fields: [
            ['account_type', AccountType],
            ['manager', 'u256'],
            ['staker', 'u256'],
            ['deposit_authority', 'u256'],
            ['withdraw_bump_seed', 'u8'],
            ['validator_list', 'u256'],
            ['reserve_stake', 'u256'],
            ['pool_mint', 'u256'],
            ['manager_fee_account', 'u256'],
            ['token_program_id', 'u256'],
            ['total_stake_lamports', 'u64'],
            ['pool_token_supply', 'u64'],
            ['last_update_epoch', 'u64'],
            ['last_update_epoch', Fee],
        ],
    })

    SCHEMA.set('ValidatorList', {
        kind: 'struct',
        fields: [
            ['account_type', AccountType],
            ['max_validators', 'u32'],
            ['validators', [ValidatorStakeInfo]]
        ],
    })

    SCHEMA.set('StakeStatus', {
        kind: 'enum',
        field: 'enum',
        values: [
            ['Active', StakeStatusEnum],
            ['DeactivatingTransient', StakeStatusEnum],
            ['ReadyForRemoval', StakeStatusEnum],
        ],
    })

    SCHEMA.set('StakeStatusEnum', { kind: 'struct', fields: [] })

    SCHEMA.set('ValidatorStakeInfo', {
        kind: 'struct',
        fields: [
            ['status', StakeStatus],
            ['vote_account_address', 'u256'],
            ['stake_lamports', 'u64'],
            ['last_update_epoch', 'u64'],
        ],
    })

    return SCHEMA
}