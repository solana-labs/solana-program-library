//! Program state processor

use crate::utils::{assert_account_owner, assert_mint_authority, assert_mint_owner_program};
use crate::{
    error::TimelockError,
    state::{
        enums::GovernanceAccountType,
        timelock_config::TimelockConfig,
        timelock_set::TimelockSet,
        timelock_state::{TimelockState, TIMELOCK_STATE_VERSION},
        timelock_state::{DESC_SIZE, NAME_SIZE},
    },
    utils::{
        assert_account_mint, assert_initialized, assert_mint_decimals, assert_mint_initialized,
        assert_rent_exempt, assert_uninitialized, get_mint_decimals, get_mint_from_token_account,
        spl_token_mint_to, TokenMintToParams,
    },
    PROGRAM_AUTHORITY_SEED,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::{rent::Rent, Sysvar},
};

/// Create a new timelock set
pub fn process_init_timelock_set(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: [u8; NAME_SIZE],
    desc_link: [u8; DESC_SIZE],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let timelock_state_account_info = next_account_info(account_info_iter)?; //0
    let timelock_set_account_info = next_account_info(account_info_iter)?; //1
    let timelock_config_account_info = next_account_info(account_info_iter)?; //2
    let signatory_mint_account_info = next_account_info(account_info_iter)?; //3
    let admin_mint_account_info = next_account_info(account_info_iter)?; //4
    let voting_mint_account_info = next_account_info(account_info_iter)?; //5
    let yes_voting_mint_account_info = next_account_info(account_info_iter)?; //6
    let no_voting_mint_account_info = next_account_info(account_info_iter)?; //7
    let signatory_validation_account_info = next_account_info(account_info_iter)?; //8
    let admin_validation_account_info = next_account_info(account_info_iter)?; //9
    let voting_validation_account_info = next_account_info(account_info_iter)?; //10
    let destination_admin_account_info = next_account_info(account_info_iter)?; //11
    let destination_sig_account_info = next_account_info(account_info_iter)?; //12
    let yes_voting_dump_account_info = next_account_info(account_info_iter)?; //13
    let no_voting_dump_account_info = next_account_info(account_info_iter)?; //14
    let source_holding_account_info = next_account_info(account_info_iter)?; //15
    let source_mint_account_info = next_account_info(account_info_iter)?; //16
    let timelock_program_authority_info = next_account_info(account_info_iter)?; //17
    let token_program_info = next_account_info(account_info_iter)?; //18
    let rent_info = next_account_info(account_info_iter)?; //19
    let rent = &Rent::from_account_info(rent_info)?;

    let mut new_timelock_state: TimelockState = assert_uninitialized(timelock_state_account_info)?;
    let mut new_timelock_set: TimelockSet = assert_uninitialized(timelock_set_account_info)?;
    let mut timelock_config: TimelockConfig = assert_initialized(timelock_config_account_info)?;

    new_timelock_set.account_type = GovernanceAccountType::Proposal;
    new_timelock_set.config = *timelock_config_account_info.key;
    new_timelock_set.token_program_id = *token_program_info.key;
    new_timelock_set.state = *timelock_state_account_info.key;
    new_timelock_set.admin_mint = *admin_mint_account_info.key;
    new_timelock_set.voting_mint = *voting_mint_account_info.key;
    new_timelock_set.yes_voting_mint = *yes_voting_mint_account_info.key;
    new_timelock_set.no_voting_mint = *no_voting_mint_account_info.key;
    new_timelock_set.source_mint = *source_mint_account_info.key;
    new_timelock_set.signatory_mint = *signatory_mint_account_info.key;
    new_timelock_set.source_holding = *source_holding_account_info.key;
    new_timelock_set.yes_voting_dump = *yes_voting_dump_account_info.key;
    new_timelock_set.no_voting_dump = *no_voting_dump_account_info.key;
    new_timelock_set.admin_validation = *admin_validation_account_info.key;
    new_timelock_set.voting_validation = *voting_validation_account_info.key;
    new_timelock_set.signatory_validation = *signatory_validation_account_info.key;
    new_timelock_state.version = TIMELOCK_STATE_VERSION;
    new_timelock_state.timelock_set = *timelock_set_account_info.key;
    new_timelock_state.desc_link = desc_link;
    new_timelock_state.name = name;
    new_timelock_state.total_signing_tokens_minted = 1;
    new_timelock_state.number_of_executed_transactions = 0;
    new_timelock_state.number_of_transactions = 0;
    timelock_config.count = match timelock_config.count.checked_add(1) {
        Some(val) => val,
        None => return Err(TimelockError::NumericalOverflow.into()),
    };

    assert_rent_exempt(rent, timelock_set_account_info)?;
    assert_rent_exempt(rent, source_holding_account_info)?;
    assert_rent_exempt(rent, admin_mint_account_info)?;
    assert_rent_exempt(rent, voting_mint_account_info)?;
    assert_rent_exempt(rent, yes_voting_mint_account_info)?;
    assert_rent_exempt(rent, no_voting_mint_account_info)?;
    assert_rent_exempt(rent, signatory_mint_account_info)?;
    assert_rent_exempt(rent, admin_validation_account_info)?;
    assert_rent_exempt(rent, signatory_validation_account_info)?;
    assert_rent_exempt(rent, voting_validation_account_info)?;

    // Cheap computational and stack-wise calls for initialization checks, no deserialization required
    assert_mint_initialized(signatory_mint_account_info)?;
    assert_mint_initialized(admin_mint_account_info)?;
    assert_mint_initialized(voting_mint_account_info)?;
    assert_mint_initialized(yes_voting_mint_account_info)?;
    assert_mint_initialized(no_voting_mint_account_info)?;
    assert_mint_initialized(source_mint_account_info)?;

    assert_mint_owner_program(signatory_mint_account_info, token_program_info.key)?;
    assert_mint_owner_program(admin_mint_account_info, token_program_info.key)?;
    assert_mint_owner_program(voting_mint_account_info, token_program_info.key)?;
    assert_mint_owner_program(yes_voting_mint_account_info, token_program_info.key)?;
    assert_mint_owner_program(no_voting_mint_account_info, token_program_info.key)?;
    assert_mint_owner_program(source_mint_account_info, token_program_info.key)?;

    let source_holding_mint: Pubkey = get_mint_from_token_account(source_holding_account_info)?;

    assert_account_mint(destination_sig_account_info, signatory_mint_account_info)?;
    assert_account_mint(destination_admin_account_info, admin_mint_account_info)?;
    assert_account_mint(
        signatory_validation_account_info,
        signatory_mint_account_info,
    )?;
    assert_account_mint(admin_validation_account_info, admin_mint_account_info)?;
    assert_account_mint(voting_validation_account_info, voting_mint_account_info)?;
    assert_account_mint(yes_voting_dump_account_info, yes_voting_mint_account_info)?;
    assert_account_mint(no_voting_dump_account_info, no_voting_mint_account_info)?;
    assert_account_mint(source_holding_account_info, source_mint_account_info)?;

    assert_account_owner(
        signatory_validation_account_info,
        timelock_program_authority_info.key,
    )?;
    assert_account_owner(
        admin_validation_account_info,
        timelock_program_authority_info.key,
    )?;
    assert_account_owner(
        voting_validation_account_info,
        timelock_program_authority_info.key,
    )?;
    assert_account_owner(
        yes_voting_dump_account_info,
        timelock_program_authority_info.key,
    )?;
    assert_account_owner(
        no_voting_dump_account_info,
        timelock_program_authority_info.key,
    )?;
    assert_account_owner(
        source_holding_account_info,
        timelock_program_authority_info.key,
    )?;

    let source_mint_decimals = get_mint_decimals(source_mint_account_info)?;
    assert_mint_decimals(voting_mint_account_info, source_mint_decimals)?;
    assert_mint_decimals(yes_voting_mint_account_info, source_mint_decimals)?;
    assert_mint_decimals(no_voting_mint_account_info, source_mint_decimals)?;

    assert_mint_authority(
        signatory_mint_account_info,
        timelock_program_authority_info.key,
    )?;
    assert_mint_authority(admin_mint_account_info, timelock_program_authority_info.key)?;
    assert_mint_authority(
        voting_mint_account_info,
        timelock_program_authority_info.key,
    )?;
    assert_mint_authority(
        yes_voting_mint_account_info,
        timelock_program_authority_info.key,
    )?;
    assert_mint_authority(
        no_voting_mint_account_info,
        timelock_program_authority_info.key,
    )?;

    if source_holding_mint != timelock_config.governance_mint {
        if let Some(council_mint) = timelock_config.council_mint {
            if source_holding_mint != council_mint {
                return Err(TimelockError::AccountsShouldMatch.into());
            }
        } else {
            return Err(TimelockError::AccountsShouldMatch.into());
        }
    }

    TimelockSet::pack(
        new_timelock_set,
        &mut timelock_set_account_info.data.borrow_mut(),
    )?;
    TimelockState::pack(
        new_timelock_state,
        &mut timelock_state_account_info.data.borrow_mut(),
    )?;
    TimelockConfig::pack(
        timelock_config,
        &mut timelock_config_account_info.data.borrow_mut(),
    )?;

    let mut seeds = vec![
        PROGRAM_AUTHORITY_SEED,
        timelock_set_account_info.key.as_ref(),
    ];

    let (authority_key, bump_seed) = Pubkey::find_program_address(&seeds[..], program_id);
    if timelock_program_authority_info.key != &authority_key {
        return Err(TimelockError::InvalidTimelockAuthority.into());
    }
    let bump = &[bump_seed];
    seeds.push(bump);
    let authority_signer_seeds = &seeds[..];

    spl_token_mint_to(TokenMintToParams {
        mint: admin_mint_account_info.clone(),
        destination: destination_admin_account_info.clone(),
        amount: 1,
        authority: timelock_program_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    spl_token_mint_to(TokenMintToParams {
        mint: signatory_mint_account_info.clone(),
        destination: destination_sig_account_info.clone(),
        amount: 1,
        authority: timelock_program_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;
    Ok(())
}
