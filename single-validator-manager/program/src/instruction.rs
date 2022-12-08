//! Instruction types

use {
    crate::{
        error::SingleValidatorManagerError,
        pda::{
            ManagerAddress, MintAddress, ReserveAddress, StakePoolAddress, ValidatorListAddress,
        },
    },
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    bytemuck::Pod,
    mpl_token_metadata::pda::find_metadata_account,
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        borsh::get_packed_len,
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        stake, system_program, sysvar,
    },
    spl_associated_token_account::get_associated_token_address_with_program_id,
    spl_stake_pool::{
        find_stake_program_address, find_transient_stake_program_address,
        find_withdraw_authority_program_address,
    },
    std::{convert::TryFrom, num::NonZeroU32},
};

/// Instructions supported by the StakePool program.
#[repr(u8)]
#[derive(Clone, Copy, Debug, TryFromPrimitive, IntoPrimitive)]
pub enum SingleValidatorManagerInstruction {
    ///   Creates a new StakePool for one validator.
    ///
    ///   The rent-exemption lamports for the stake pool, validator list, pool
    ///   mint, and fee account amount to 12,388,800 lamports, or ~0.0124 SOL.
    ///
    ///   0. `[sw]` Payer for the stake pool accounts
    ///   1. `[]` Validator vote account for the single validator stake pool
    ///   2. `[w]` Uninitialized stake pool to create.
    ///   3. `[w]` Uninitialized validator stake list storage account
    ///   4. `[w]` Uninitialized reserve stake account.
    ///   5. `[w]` Uninitialized pool token mint.
    ///   6. `[w]` Uninitialized pool token account to collect the fees.
    ///   7. `[]` Manager
    ///   8. `[]` Stake pool withdraw authority
    ///   9. `[]` System program id
    ///  10. `[]` Stake program id
    ///  11. `[]` Token program id
    ///  12. `[]` Stake pool program id
    CreateStakePool,

    ///   Adds the single validator stake account to the pool.
    ///
    ///   The stake account will have the rent-exempt amount plus
    ///   `max(crate::MINIMUM_ACTIVE_STAKE, solana_program::stake::tools::get_minimum_delegation())`.
    ///   It is funded from the stake pool reserve.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[]` Staker
    ///   2. `[w]` Reserve stake account
    ///   3. `[]` Stake pool withdraw authority
    ///   4. `[w]` Validator stake list storage account
    ///   5. `[w]` Stake account to add to the pool
    ///   6. `[]` Validator vote account for the single validator stake pool
    ///   7. `[]` Rent sysvar
    ///   8. `[]` Clock sysvar
    ///   9. '[]' Stake history sysvar
    ///  10. '[]' Stake config sysvar
    ///  11. `[]` System program
    ///  12. `[]` Stake program
    ///  13. `[]` Stake pool program
    ///
    ///  userdata: optional non-zero u32 seed used for generating the validator
    ///  stake address
    AddValidatorToPool,

    ///   Removes a non-canonical validator from the pool, deactivating its stake
    ///
    ///   This is useful if an existing stake pool is assigned to the management
    ///   program, but has additional validators.
    ///
    ///   0. `[]` Validator vote account for the single validator stake pool
    ///   1. `[w]` Stake pool
    ///   2. `[]` Staker
    ///   3. `[]` Stake pool withdraw authority
    ///   4. `[w]` Validator stake list storage account
    ///   5. `[w]` Stake account to remove from the pool
    ///   6. `[]` Transient stake account, to check that that we're not trying to activate
    ///   7. `[]` Sysvar clock
    ///   8. `[]` Stake program
    ///   9. `[]` Stake pool program
    RemoveValidatorFromPool,

    /// Decrease active stake on a validator, eventually moving it to the reserve
    ///
    /// The instruction only succeeds if the stake pool reserve has less than
    /// the minimum delegation amount. It allows small pool token holders to
    /// withdraw their tokens from the pool.
    ///
    /// This instruction can also be used on additional validators that are not
    /// the single validator, if an existing stake pool was reassigned to this
    /// management program.
    ///
    ///  0. `[]` Validator vote account for the single validator stake pool
    ///  1. `[]` Stake pool
    ///  2. `[]` Stake pool staker
    ///  3. `[]` Stake pool withdraw authority
    ///  4. `[w]` Validator list
    ///  5. `[w]` Canonical stake account to split from
    ///  6. `[w]` Transient stake account to receive split
    ///  7. `[]` Clock sysvar
    ///  8. `[]` Rent sysvar
    ///  9. `[]` System program
    /// 10. `[]` Stake program
    /// 11. `[]` Stake pool program
    ///
    /// userdata: seed to use for transient stake account
    DecreaseValidatorStake,

    /// Increase stake on the validator from the reserve account
    ///
    /// This instruction only succeeds if the stake pool reserve has more than
    /// the `2 * (minimum_delegation_amount + rent_exemption) + rent_exemption`,
    /// and will activate enough to leave `minimum_delegation_amount + 2 * rent_exemption`.
    ///
    ///  0. `[]` Stake pool
    ///  1. `[]` Stake pool staker
    ///  2. `[]` Stake pool withdraw authority
    ///  3. `[w]` Validator list
    ///  4. `[w]` Stake pool reserve stake
    ///  5. `[w]` Transient stake account
    ///  6. `[]` Validator stake account
    ///  7. `[]` Validator vote account for the single validator stake pool
    ///  8. '[]' Clock sysvar
    ///  9. '[]' Rent sysvar
    /// 10. `[]` Stake History sysvar
    /// 11. `[]` Stake Config sysvar
    /// 12. `[]` System program
    /// 13. `[]` Stake program
    /// 14. `[]` Stake pool program
    ///
    ///  userdata: seed to use for transient stake account
    IncreaseValidatorStake,

    /// Burn collected fee tokens, increasing the value of the pool tokens
    ///
    /// Fees are required on SOL deposits to avoid an attack vector to leech
    /// value from the pool, by depositing SOL and immediately withdrawing a
    /// stake account.
    ///
    /// By default, fees are collected into a token account owned by the program,
    /// and are meant to be burned to increase the value of pool tokens on SOL
    /// deposit.
    ///
    /// After burning fees, users should run `update_stake_pool_balance` to
    /// accurately reflect the increased value of their pool tokens.
    ///
    /// 0. `[]` Validator that the pool should be delegated to
    /// 1. `[]` Stake pool
    /// 2. `[]` Manager
    /// 3. `[w]` Fee account
    /// 4. `[w]` Pool mint
    /// 5. `[]` Token program id
    /// 6. `[]` Stake pool program id
    BurnFees,

    ///  Update fees to the amount required by the program
    ///
    ///  This is useful for existing pools that are assigned to the management
    ///  program, and are configured with different fees.
    ///
    ///  0. `[]` Validator that the pool should be delegated to
    ///  1. `[w]` StakePool
    ///  2. `[]` Manager
    ///  3. `[]` Stake pool program
    ResetFees,

    ///  Remove funding authorities, if already set.
    ///
    ///  Single-validator pool should have no restrictions on deposits or
    ///  withdrawals. This is useful for existing pools that are assigned to the
    ///  management program, and are configured with funding authorities.
    ///
    ///  0. `[]` Validator that the pool should be delegated to
    ///  1. `[w]` StakePool
    ///  2. `[]` Manager
    ///  3. `[]` Stake pool program
    RemoveFundingAuthorities,

    /// Create token metadata for the stake-pool token in the metaplex-token program.
    ///
    /// Must be signed by the authorized voter for the validator.
    ///
    ///  0. `[s]` Authorized voter for validator
    ///  1. `[]` Validator that the pool should be delegated to
    ///  2. `[]` Stake pool
    ///  3. `[]` Manager
    ///  4. `[]` Stake pool withdraw authority
    ///  5. `[]` Pool token mint account
    ///  6. `[s, w]` Payer for creation of token metadata account
    ///  7. `[w]` Token metadata account
    ///  8. `[]` Metadata program id
    ///  9. `[]` System program id
    /// 10. `[]` Rent sysvar
    /// 11. `[]` Stake pool program id
    ///
    /// userdata: new `TokenMetadata`
    CreateTokenMetadata,

    /// Update token metadata for the stake-pool token in the metaplex-token program.
    ///
    /// Must be signed by the authorized voter for the validator.
    ///
    /// 0. `[s]` Authorized voter for validator
    /// 1. `[]` Validator that the pool should be delegated to
    /// 2. `[]` Stake pool
    /// 3. `[]` Manager
    /// 4. `[]` Stake pool withdraw authority
    /// 5. `[w]` Token metadata account
    /// 6. `[]` Metadata program id
    /// 7. `[]` Stake pool program id
    ///
    /// userdata: new `TokenMetadata`
    UpdateTokenMetadata,
}

/// Struct used for creating and updating the pool token metadata, needs to use
/// Borsh encoding to work with Metaplex token metadata
#[derive(Clone, Debug, BorshSchema, BorshSerialize, BorshDeserialize)]
pub struct TokenMetadata {
    /// Token name
    pub name: String,
    /// Token symbol e.g. stkSOL
    pub symbol: String,
    /// URI of the uploaded metadata of the spl-token
    pub uri: String,
}

/// Utility function for decoding just the instruction type
pub fn decode_instruction_type<T: TryFrom<u8>>(input: &[u8]) -> Result<T, ProgramError> {
    if input.is_empty() {
        Err(ProgramError::InvalidInstructionData)
    } else {
        T::try_from(input[0]).map_err(|_| SingleValidatorManagerError::InvalidInstruction.into())
    }
}

/// Utility function for decoding instruction data
///
/// Note: This function expects the entire instruction input, including the
/// instruction type as the first byte.  This makes the code concise and safe
/// at the expense of clarity, allowing flows such as:
///
/// match decode_instruction_type(input)? {
///     InstructionType::First => {
///         let FirstData { ... } = decode_instruction_data(input)?;
///     }
/// }
pub fn decode_instruction_data<T: Pod>(input_with_type: &[u8]) -> Result<&T, ProgramError> {
    if input_with_type.len() != std::mem::size_of::<T>().saturating_add(1) {
        Err(ProgramError::InvalidInstructionData)
    } else {
        bytemuck::try_from_bytes(&input_with_type[1..]).map_err(|_| ProgramError::InvalidArgument)
    }
}

/// Utility function for decoding instruction data serialized with Borsh
pub fn decode_instruction_data_with_borsh<T: BorshDeserialize + BorshSchema>(
    input_with_type: &[u8],
) -> Result<T, ProgramError> {
    if input_with_type.len() != get_packed_len::<T>().saturating_add(1) {
        Err(ProgramError::InvalidInstructionData)
    } else {
        T::try_from_slice(&input_with_type[1..]).map_err(|_| ProgramError::InvalidArgument)
    }
}

/// Utility function for encoding bytemuck instruction data
pub(crate) fn encode_instruction<T: Pod>(
    program_id: &Pubkey,
    accounts: Vec<AccountMeta>,
    instruction_type: SingleValidatorManagerInstruction,
    instruction_data: &T,
) -> Instruction {
    let mut data = vec![];
    data.push(u8::from(instruction_type));
    data.extend_from_slice(bytemuck::bytes_of(instruction_data));
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Utility function for encoding borsh instruction data
pub(crate) fn encode_instruction_with_borsh<T: BorshSerialize>(
    program_id: &Pubkey,
    accounts: Vec<AccountMeta>,
    instruction_type: SingleValidatorManagerInstruction,
    instruction_data: &T,
) -> Instruction {
    let mut data = vec![];
    data.push(u8::from(instruction_type));
    data.append(&mut instruction_data.try_to_vec().unwrap());
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates an 'create_stake_pool' instruction.
pub fn create_stake_pool(
    program_id: &Pubkey,
    payer: &Pubkey,
    validator: &Pubkey,
    stake_pool_program_id: &Pubkey,
) -> Instruction {
    let (stake_pool, _) = StakePoolAddress::find(program_id, validator, stake_pool_program_id);
    let (validator_list, _) =
        ValidatorListAddress::find(program_id, validator, stake_pool_program_id);
    let (reserve, _) = ReserveAddress::find(program_id, validator, stake_pool_program_id);
    let (mint, _) = MintAddress::find(program_id, validator, stake_pool_program_id);
    let (manager, _) = ManagerAddress::find(program_id, validator, stake_pool_program_id);
    let fee_account =
        get_associated_token_address_with_program_id(&manager, &mint, &spl_token::id());
    let (stake_pool_withdraw_authority, _) =
        find_withdraw_authority_program_address(stake_pool_program_id, &stake_pool);
    let accounts = vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(*validator, false),
        AccountMeta::new(stake_pool, false),
        AccountMeta::new(validator_list, false),
        AccountMeta::new(reserve, false),
        AccountMeta::new(mint, false),
        AccountMeta::new(fee_account, false),
        AccountMeta::new_readonly(manager, false),
        AccountMeta::new_readonly(stake_pool_withdraw_authority, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(*stake_pool_program_id, false),
    ];
    encode_instruction(
        program_id,
        accounts,
        SingleValidatorManagerInstruction::CreateStakePool,
        &(),
    )
}

/// Creates `AddValidatorToPool` instruction
pub fn add_validator_to_pool(
    program_id: &Pubkey,
    validator: &Pubkey,
    stake_pool_program_id: &Pubkey,
    seed: Option<NonZeroU32>,
) -> Instruction {
    let (stake_pool, _) = StakePoolAddress::find(program_id, validator, stake_pool_program_id);
    let (validator_list, _) =
        ValidatorListAddress::find(program_id, validator, stake_pool_program_id);
    let (reserve, _) = ReserveAddress::find(program_id, validator, stake_pool_program_id);
    let (manager, _) = ManagerAddress::find(program_id, validator, stake_pool_program_id);
    let (stake_pool_withdraw_authority, _) =
        find_withdraw_authority_program_address(stake_pool_program_id, &stake_pool);
    let (stake, _) =
        find_stake_program_address(stake_pool_program_id, validator, &stake_pool, seed);
    let accounts = vec![
        AccountMeta::new(stake_pool, false),
        AccountMeta::new_readonly(manager, false),
        AccountMeta::new(reserve, false),
        AccountMeta::new_readonly(stake_pool_withdraw_authority, false),
        AccountMeta::new(validator_list, false),
        AccountMeta::new(stake, false),
        AccountMeta::new_readonly(*validator, false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(stake::config::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
        AccountMeta::new_readonly(*stake_pool_program_id, false),
    ];
    encode_instruction(
        program_id,
        accounts,
        SingleValidatorManagerInstruction::AddValidatorToPool,
        &seed.map(|s| s.get()).unwrap_or(0),
    )
}

/// Creates `RemoveValidatorFromPool` instruction
pub fn remove_validator_from_pool(
    program_id: &Pubkey,
    validator: &Pubkey,
    stake_pool_program_id: &Pubkey,
    validator_to_remove: &Pubkey,
    validator_stake_seed: Option<NonZeroU32>,
    transient_stake_seed: u64,
) -> Instruction {
    let (stake_pool, _) = StakePoolAddress::find(program_id, validator, stake_pool_program_id);
    let (validator_list, _) =
        ValidatorListAddress::find(program_id, validator, stake_pool_program_id);
    let (manager, _) = ManagerAddress::find(program_id, validator, stake_pool_program_id);
    let (stake_pool_withdraw_authority, _) =
        find_withdraw_authority_program_address(stake_pool_program_id, &stake_pool);
    let (stake, _) = find_stake_program_address(
        stake_pool_program_id,
        validator_to_remove,
        &stake_pool,
        validator_stake_seed,
    );
    let (transient_stake, _) = find_transient_stake_program_address(
        stake_pool_program_id,
        validator_to_remove,
        &stake_pool,
        transient_stake_seed,
    );
    let accounts = vec![
        AccountMeta::new_readonly(*validator, false),
        AccountMeta::new(stake_pool, false),
        AccountMeta::new_readonly(manager, false),
        AccountMeta::new_readonly(stake_pool_withdraw_authority, false),
        AccountMeta::new(validator_list, false),
        AccountMeta::new(stake, false),
        AccountMeta::new_readonly(transient_stake, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
        AccountMeta::new_readonly(*stake_pool_program_id, false),
    ];
    encode_instruction(
        program_id,
        accounts,
        SingleValidatorManagerInstruction::RemoveValidatorFromPool,
        &(),
    )
}

/// Creates `DecreaseValidatorStake` instruction (rebalance from main validator account to
/// transient account)
pub fn decrease_validator_stake(
    program_id: &Pubkey,
    validator: &Pubkey,
    stake_pool_program_id: &Pubkey,
    validator_to_decrease: &Pubkey,
    validator_stake_seed: Option<NonZeroU32>,
    transient_stake_seed: u64,
) -> Instruction {
    let (stake_pool, _) = StakePoolAddress::find(program_id, validator, stake_pool_program_id);
    let (validator_list, _) =
        ValidatorListAddress::find(program_id, validator, stake_pool_program_id);
    let (reserve, _) = ReserveAddress::find(program_id, validator, stake_pool_program_id);
    let (manager, _) = ManagerAddress::find(program_id, validator, stake_pool_program_id);
    let (stake_pool_withdraw_authority, _) =
        find_withdraw_authority_program_address(stake_pool_program_id, &stake_pool);
    let (stake, _) = find_stake_program_address(
        stake_pool_program_id,
        validator_to_decrease,
        &stake_pool,
        validator_stake_seed,
    );
    let (transient_stake, _) = find_transient_stake_program_address(
        stake_pool_program_id,
        validator_to_decrease,
        &stake_pool,
        transient_stake_seed,
    );

    let accounts = vec![
        AccountMeta::new_readonly(*validator, false),
        AccountMeta::new_readonly(stake_pool, false),
        AccountMeta::new_readonly(manager, false),
        AccountMeta::new_readonly(stake_pool_withdraw_authority, false),
        AccountMeta::new(validator_list, false),
        AccountMeta::new(stake, false),
        AccountMeta::new(transient_stake, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
        AccountMeta::new_readonly(*stake_pool_program_id, false),
    ];
    encode_instruction(
        program_id,
        accounts,
        SingleValidatorManagerInstruction::DecreaseValidatorStake,
        &transient_stake_seed,
    )
}

/// Creates `IncreaseValidatorStake` instruction (rebalance from reserve account to
/// transient account)
pub fn increase_validator_stake(
    program_id: &Pubkey,
    validator: &Pubkey,
    stake_pool_program_id: &Pubkey,
    validator_stake_seed: Option<NonZeroU32>,
    transient_stake_seed: u64,
) -> Instruction {
    let (stake_pool, _) = StakePoolAddress::find(program_id, validator, stake_pool_program_id);
    let (validator_list, _) =
        ValidatorListAddress::find(program_id, validator, stake_pool_program_id);
    let (reserve, _) = ReserveAddress::find(program_id, validator, stake_pool_program_id);
    let (manager, _) = ManagerAddress::find(program_id, validator, stake_pool_program_id);
    let (stake_pool_withdraw_authority, _) =
        find_withdraw_authority_program_address(stake_pool_program_id, &stake_pool);
    let (stake, _) = find_stake_program_address(
        stake_pool_program_id,
        validator,
        &stake_pool,
        validator_stake_seed,
    );
    let (transient_stake, _) = find_transient_stake_program_address(
        stake_pool_program_id,
        validator,
        &stake_pool,
        transient_stake_seed,
    );
    let accounts = vec![
        AccountMeta::new_readonly(stake_pool, false),
        AccountMeta::new_readonly(manager, false),
        AccountMeta::new_readonly(stake_pool_withdraw_authority, false),
        AccountMeta::new(validator_list, false),
        AccountMeta::new(reserve, false),
        AccountMeta::new(transient_stake, false),
        AccountMeta::new_readonly(stake, false),
        AccountMeta::new_readonly(*validator, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::stake_history::id(), false),
        AccountMeta::new_readonly(stake::config::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(stake::program::id(), false),
        AccountMeta::new_readonly(*stake_pool_program_id, false),
    ];
    encode_instruction(
        program_id,
        accounts,
        SingleValidatorManagerInstruction::IncreaseValidatorStake,
        &transient_stake_seed,
    )
}

/// Creates a `ResetFees` instruction.
pub fn reset_fees(
    program_id: &Pubkey,
    validator: &Pubkey,
    stake_pool_program_id: &Pubkey,
) -> Instruction {
    let (stake_pool, _) = StakePoolAddress::find(program_id, validator, stake_pool_program_id);
    let (manager, _) = ManagerAddress::find(program_id, validator, stake_pool_program_id);
    let accounts = vec![
        AccountMeta::new_readonly(*validator, false),
        AccountMeta::new(stake_pool, false),
        AccountMeta::new_readonly(manager, false),
        AccountMeta::new_readonly(*stake_pool_program_id, false),
    ];
    encode_instruction(
        program_id,
        accounts,
        SingleValidatorManagerInstruction::ResetFees,
        &(),
    )
}

/// Creates a `RemoveFundingAuthorities` instruction.
pub fn remove_funding_authorities(
    program_id: &Pubkey,
    validator: &Pubkey,
    stake_pool_program_id: &Pubkey,
) -> Instruction {
    let (stake_pool, _) = StakePoolAddress::find(program_id, validator, stake_pool_program_id);
    let (manager, _) = ManagerAddress::find(program_id, validator, stake_pool_program_id);
    let accounts = vec![
        AccountMeta::new_readonly(*validator, false),
        AccountMeta::new(stake_pool, false),
        AccountMeta::new_readonly(manager, false),
        AccountMeta::new_readonly(*stake_pool_program_id, false),
    ];
    encode_instruction(
        program_id,
        accounts,
        SingleValidatorManagerInstruction::RemoveFundingAuthorities,
        &(),
    )
}

/// Creates an instruction to update metadata in the mpl token metadata program account for
/// the pool token
pub fn update_token_metadata(
    program_id: &Pubkey,
    authorized_voter: &Pubkey,
    validator: &Pubkey,
    stake_pool_program_id: &Pubkey,
    name: String,
    symbol: String,
    uri: String,
) -> Instruction {
    let (stake_pool, _) = StakePoolAddress::find(program_id, validator, stake_pool_program_id);
    let (manager, _) = ManagerAddress::find(program_id, validator, stake_pool_program_id);
    let (mint, _) = MintAddress::find(program_id, validator, stake_pool_program_id);
    let (stake_pool_withdraw_authority, _) =
        find_withdraw_authority_program_address(program_id, &stake_pool);
    let (token_metadata, _) = find_metadata_account(&mint);
    let accounts = vec![
        AccountMeta::new_readonly(*authorized_voter, true),
        AccountMeta::new_readonly(*validator, false),
        AccountMeta::new_readonly(stake_pool, false),
        AccountMeta::new_readonly(manager, false),
        AccountMeta::new_readonly(stake_pool_withdraw_authority, false),
        AccountMeta::new(token_metadata, false),
        AccountMeta::new_readonly(mpl_token_metadata::id(), false),
        AccountMeta::new_readonly(*stake_pool_program_id, false),
    ];
    encode_instruction_with_borsh(
        program_id,
        accounts,
        SingleValidatorManagerInstruction::UpdateTokenMetadata,
        &TokenMetadata { name, symbol, uri },
    )
}

/// Creates an instruction to create metadata using the mpl token metadata program for
/// the pool token
pub fn create_token_metadata(
    program_id: &Pubkey,
    authorized_voter: &Pubkey,
    validator: &Pubkey,
    stake_pool_program_id: &Pubkey,
    payer: &Pubkey,
    name: String,
    symbol: String,
    uri: String,
) -> Instruction {
    let (stake_pool, _) = StakePoolAddress::find(program_id, validator, stake_pool_program_id);
    let (manager, _) = ManagerAddress::find(program_id, validator, stake_pool_program_id);
    let (mint, _) = MintAddress::find(program_id, validator, stake_pool_program_id);
    let (stake_pool_withdraw_authority, _) =
        find_withdraw_authority_program_address(program_id, &stake_pool);
    let (token_metadata, _) = find_metadata_account(&mint);
    let accounts = vec![
        AccountMeta::new_readonly(*authorized_voter, true),
        AccountMeta::new_readonly(*validator, false),
        AccountMeta::new_readonly(stake_pool, false),
        AccountMeta::new_readonly(manager, false),
        AccountMeta::new_readonly(stake_pool_withdraw_authority, false),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new(token_metadata, false),
        AccountMeta::new_readonly(mpl_token_metadata::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(*stake_pool_program_id, false),
    ];
    encode_instruction_with_borsh(
        program_id,
        accounts,
        SingleValidatorManagerInstruction::CreateTokenMetadata,
        &TokenMetadata { name, symbol, uri },
    )
}

#[cfg(test)]
mod test {
    use {
        super::*,
        solana_program::{
            borsh::get_instance_packed_len, program_pack::Pack, rent::Rent,
            stake::state::StakeState,
        },
        spl_stake_pool::state::{StakePool, ValidatorList},
        spl_token_2022::state::{Account, Mint},
    };

    #[test]
    fn test_total_pool_rent() {
        let rent = Rent::default();
        let sizes = vec![
            get_packed_len::<StakePool>(),
            get_instance_packed_len(&ValidatorList::new(1)).unwrap(),
            std::mem::size_of::<StakeState>(),
            Mint::LEN,
            Account::LEN,
        ];
        assert_eq!(
            sizes
                .into_iter()
                .map(|x| rent.minimum_balance(x))
                .sum::<u64>(),
            12_388_800
        );
    }
}
