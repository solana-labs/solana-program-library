//! Program state processor

use {
    crate::instruction::{
        decode_instruction_data, decode_instruction_data_with_borsh, decode_instruction_type,
        SingleValidatorManagerInstruction, TokenMetadata,
    },
    mpl_token_metadata::{
        instruction::{create_metadata_accounts_v3, update_metadata_accounts_v2},
        pda::find_metadata_account,
        state::DataV2,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::try_from_slice_unchecked,
        clock::{Clock, Epoch},
        decode_error::DecodeError,
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        pubkey::Pubkey,
        rent::Rent,
        stake, system_instruction, system_program,
        sysvar::Sysvar,
    },
    spl_token_2022::{
        check_spl_token_program_account,
        extension::{BaseStateWithExtensions, StateWithExtensions},
        state::Mint,
    },
    std::num::NonZeroU32,
};

/// Check system program address
fn check_system_program(program_id: &Pubkey) -> Result<(), ProgramError> {
    if *program_id != system_program::id() {
        msg!(
            "Expected system program {}, received {}",
            system_program::id(),
            program_id
        );
        Err(ProgramError::IncorrectProgramId)
    } else {
        Ok(())
    }
}

/// Check stake program address
fn check_stake_program(program_id: &Pubkey) -> Result<(), ProgramError> {
    if *program_id != stake::program::id() {
        msg!(
            "Expected stake program {}, received {}",
            stake::program::id(),
            program_id
        );
        Err(ProgramError::IncorrectProgramId)
    } else {
        Ok(())
    }
}

/// Check mpl metadata program
fn check_mpl_metadata_program(program_id: &Pubkey) -> Result<(), ProgramError> {
    if *program_id != mpl_token_metadata::id() {
        msg!(
            "Expected mpl metadata program {}, received {}",
            mpl_token_metadata::id(),
            program_id
        );
        Err(ProgramError::IncorrectProgramId)
    } else {
        Ok(())
    }
}

/// Check rent sysvar correctness
fn check_rent_sysvar(sysvar_key: &Pubkey) -> Result<(), ProgramError> {
    if *sysvar_key != solana_program::sysvar::rent::id() {
        msg!(
            "Expected rent sysvar {}, received {}",
            solana_program::sysvar::rent::id(),
            sysvar_key
        );
        Err(ProgramError::InvalidArgument)
    } else {
        Ok(())
    }
}

/// Check account owner is the given program
fn check_account_owner(
    account_info: &AccountInfo,
    program_id: &Pubkey,
) -> Result<(), ProgramError> {
    if *program_id != *account_info.owner {
        msg!(
            "Expected account to be owned by program {}, received {}",
            program_id,
            account_info.owner
        );
        Err(ProgramError::IncorrectProgramId)
    } else {
        Ok(())
    }
}

/// Create a stake account on a PDA without transferring lamports
fn create_stake_account<'a>(
    stake_account_info: AccountInfo<'a>,
    stake_account_signer_seeds: &[&[u8]],
    system_program_info: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    invoke_signed(
        &system_instruction::allocate(
            stake_account_info.key,
            std::mem::size_of::<stake::state::StakeState>() as u64,
        ),
        &[stake_account_info.clone(), system_program_info.clone()],
        &[stake_account_signer_seeds],
    )?;
    invoke_signed(
        &system_instruction::assign(stake_account_info.key, &stake::program::id()),
        &[stake_account_info, system_program_info],
        &[stake_account_signer_seeds],
    )
}

/// Issue stake::instruction::authorize instructions to update both authorities
fn stake_authorize<'a>(
    stake_account: AccountInfo<'a>,
    stake_authority: AccountInfo<'a>,
    new_stake_authority: &Pubkey,
    clock: AccountInfo<'a>,
    stake_program_info: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let authorize_instruction = stake::instruction::authorize(
        stake_account.key,
        stake_authority.key,
        new_stake_authority,
        stake::state::StakeAuthorize::Staker,
        None,
    );

    invoke(
        &authorize_instruction,
        &[
            stake_account.clone(),
            clock.clone(),
            stake_authority.clone(),
            stake_program_info.clone(),
        ],
    )?;

    let authorize_instruction = stake::instruction::authorize(
        stake_account.key,
        stake_authority.key,
        new_stake_authority,
        stake::state::StakeAuthorize::Withdrawer,
        None,
    );

    invoke(
        &authorize_instruction,
        &[stake_account, clock, stake_authority, stake_program_info],
    )
}

/// Issue stake::instruction::authorize instructions to update both authorities
#[allow(clippy::too_many_arguments)]
fn stake_authorize_signed<'a>(
    stake_pool: &Pubkey,
    stake_account: AccountInfo<'a>,
    stake_authority: AccountInfo<'a>,
    authority_type: &[u8],
    bump_seed: u8,
    new_stake_authority: &Pubkey,
    clock: AccountInfo<'a>,
    stake_program_info: AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let me_bytes = stake_pool.to_bytes();
    let authority_signature_seeds = [&me_bytes[..32], authority_type, &[bump_seed]];
    let signers = &[&authority_signature_seeds[..]];

    let authorize_instruction = stake::instruction::authorize(
        stake_account.key,
        stake_authority.key,
        new_stake_authority,
        stake::state::StakeAuthorize::Staker,
        None,
    );

    invoke_signed(
        &authorize_instruction,
        &[
            stake_account.clone(),
            clock.clone(),
            stake_authority.clone(),
            stake_program_info.clone(),
        ],
        signers,
    )?;

    let authorize_instruction = stake::instruction::authorize(
        stake_account.key,
        stake_authority.key,
        new_stake_authority,
        stake::state::StakeAuthorize::Withdrawer,
        None,
    );
    invoke_signed(
        &authorize_instruction,
        &[stake_account, clock, stake_authority, stake_program_info],
        signers,
    )
}

/// Issue a spl_token `Burn` instruction.
#[allow(clippy::too_many_arguments)]
fn token_burn<'a>(
    token_program: AccountInfo<'a>,
    burn_account: AccountInfo<'a>,
    mint: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    amount: u64,
) -> Result<(), ProgramError> {
    let ix = spl_token_2022::instruction::burn(
        token_program.key,
        burn_account.key,
        mint.key,
        authority.key,
        &[],
        amount,
    )?;

    invoke(&ix, &[burn_account, mint, authority, token_program])
}

/// Processes `CreateStakePool` instruction.
fn process_create_stake_pool(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    Ok(())
}

/// Processes `AddValidatorToPool` instruction.
fn process_add_validator_to_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    raw_validator_seed: &u32,
) -> ProgramResult {
    Ok(())
}

/// Processes `RemoveValidatorFromPool` instruction.
fn process_remove_validator_from_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    Ok(())
}

/// Processes `DecreaseValidatorStake` instruction.
fn process_decrease_validator_stake(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    transient_stake_seed: &u64,
) -> ProgramResult {
    Ok(())
}

/// Processes `IncreaseValidatorStake` instruction.
fn process_increase_validator_stake(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    transient_stake_seed: &u64,
) -> ProgramResult {
    Ok(())
}

fn process_create_pool_token_metadata(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    token_metadata: TokenMetadata,
) -> ProgramResult {
    Ok(())
}

fn process_update_pool_token_metadata(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    token_metadata: TokenMetadata,
) -> ProgramResult {
    Ok(())
}

/// Processes [ResetFees](enum.Instruction.html).
fn process_reset_fees(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    Ok(())
}

/// Processes [RemoveFundingAuthorities](enum.Instruction.html).
fn process_remove_funding_authorities(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    Ok(())
}

/// Processes [BurnFees](enum.Instruction.html).
fn process_burn_fees(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    Ok(())
}

/// Processes [Instruction](enum.Instruction.html).
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    match decode_instruction_type(input)? {
        SingleValidatorManagerInstruction::CreateStakePool => {
            msg!("Instruction: Create stake pool");
            process_create_stake_pool(program_id, accounts)
        }
        SingleValidatorManagerInstruction::AddValidatorToPool => {
            msg!("Instruction: AddValidatorToPool");
            let seed = decode_instruction_data::<u32>(input)?;
            process_add_validator_to_pool(program_id, accounts, seed)
        }
        SingleValidatorManagerInstruction::RemoveValidatorFromPool => {
            msg!("Instruction: RemoveValidatorFromPool");
            process_remove_validator_from_pool(program_id, accounts)
        }
        SingleValidatorManagerInstruction::DecreaseValidatorStake => {
            msg!("Instruction: DecreaseValidatorStake");
            let transient_stake_seed = decode_instruction_data::<u64>(input)?;
            process_decrease_validator_stake(program_id, accounts, transient_stake_seed)
        }
        SingleValidatorManagerInstruction::IncreaseValidatorStake => {
            msg!("Instruction: IncreaseValidatorStake");
            let transient_stake_seed = decode_instruction_data::<u64>(input)?;
            process_increase_validator_stake(program_id, accounts, transient_stake_seed)
        }
        SingleValidatorManagerInstruction::ResetFees => {
            msg!("Instruction: ResetFees");
            process_reset_fees(program_id, accounts)
        }
        SingleValidatorManagerInstruction::RemoveFundingAuthorities => {
            msg!("Instruction: RemoveFundingAuthorities");
            process_remove_funding_authorities(program_id, accounts)
        }
        SingleValidatorManagerInstruction::BurnFees => {
            msg!("Instruction: BurnFees");
            process_burn_fees(program_id, accounts)
        }
        SingleValidatorManagerInstruction::CreateTokenMetadata => {
            msg!("Instruction: CreateTokenMetadata");
            let token_metadata = decode_instruction_data_with_borsh::<TokenMetadata>(input)?;
            process_create_pool_token_metadata(program_id, accounts, token_metadata)
        }
        SingleValidatorManagerInstruction::UpdateTokenMetadata => {
            msg!("Instruction: UpdateTokenMetadata");
            let token_metadata = decode_instruction_data_with_borsh::<TokenMetadata>(input)?;
            process_update_pool_token_metadata(program_id, accounts, token_metadata)
        }
    }
}
