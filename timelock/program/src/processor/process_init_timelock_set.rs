//! Program state processor

use crate::{
    error::TimelockError,
    state::{
        timelock_config::TimelockConfig,
        timelock_program::TimelockProgram,
        timelock_set::{TimelockSet, TIMELOCK_SET_VERSION},
        timelock_state::{TimelockState, TIMELOCK_STATE_VERSION},
        timelock_state::{DESC_SIZE, NAME_SIZE},
    },
    utils::{
        assert_cheap_mint_initialized, assert_initialized, assert_mint_matching,
        assert_rent_exempt, assert_token_program_is_correct, assert_uninitialized,
        get_mint_from_account, spl_token_mint_to, TokenMintToParams,
    },
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
    let timelock_state_account_info = next_account_info(account_info_iter)?;
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let timelock_config_account_info = next_account_info(account_info_iter)?;
    let signatory_mint_account_info = next_account_info(account_info_iter)?;
    let admin_mint_account_info = next_account_info(account_info_iter)?;
    let voting_mint_account_info = next_account_info(account_info_iter)?;
    let yes_voting_mint_account_info = next_account_info(account_info_iter)?;
    let no_voting_mint_account_info = next_account_info(account_info_iter)?;
    let signatory_validation_account_info = next_account_info(account_info_iter)?;
    let admin_validation_account_info = next_account_info(account_info_iter)?;
    let voting_validation_account_info = next_account_info(account_info_iter)?;
    let destination_admin_account_info = next_account_info(account_info_iter)?;
    let destination_sig_account_info = next_account_info(account_info_iter)?;
    let yes_voting_dump_account_info = next_account_info(account_info_iter)?;
    let no_voting_dump_account_info = next_account_info(account_info_iter)?;
    let source_holding_account_info = next_account_info(account_info_iter)?;
    let source_mint_account_info = next_account_info(account_info_iter)?;
    let timelock_program_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_info = next_account_info(account_info_iter)?;
    let token_program_info = next_account_info(account_info_iter)?;
    let rent_info = next_account_info(account_info_iter)?;
    let rent = &Rent::from_account_info(rent_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_info)?;
    let mut timelock_config: TimelockConfig = assert_initialized(timelock_config_account_info)?;

    let mut new_timelock_state: TimelockState = assert_uninitialized(timelock_state_account_info)?;
    let mut new_timelock_set: TimelockSet = assert_uninitialized(timelock_set_account_info)?;
    new_timelock_set.version = TIMELOCK_SET_VERSION;
    new_timelock_set.config = *timelock_config_account_info.key;
    new_timelock_set.state = *timelock_state_account_info.key;
    new_timelock_set.admin_mint = *admin_mint_account_info.key;
    new_timelock_set.voting_mint = *voting_mint_account_info.key;
    new_timelock_set.yes_voting_mint = *yes_voting_mint_account_info.key;
    new_timelock_set.no_voting_mint = *no_voting_mint_account_info.key;
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
    new_timelock_state.executions = 0;
    new_timelock_state.used_txn_slots = 0;
    timelock_config.count = match timelock_config.count.checked_add(1) {
        Some(val) => val,
        None => return Err(TimelockError::NumericalOverflow.into()),
    };

    assert_token_program_is_correct(&timelock_program, token_program_info)?;

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

    // Cheap computational and stack-wise calls for initialization checks, no deserialization req'd
    assert_cheap_mint_initialized(admin_mint_account_info)?;
    assert_cheap_mint_initialized(voting_mint_account_info)?;
    assert_cheap_mint_initialized(yes_voting_mint_account_info)?;
    assert_cheap_mint_initialized(no_voting_mint_account_info)?;
    assert_cheap_mint_initialized(signatory_mint_account_info)?;
    assert_cheap_mint_initialized(source_mint_account_info)?;

    let source_holding_mint: Pubkey = get_mint_from_account(source_holding_account_info)?;

    assert_mint_matching(destination_sig_account_info, signatory_mint_account_info)?;
    assert_mint_matching(destination_admin_account_info, admin_mint_account_info)?;
    assert_mint_matching(
        signatory_validation_account_info,
        signatory_mint_account_info,
    )?;
    assert_mint_matching(admin_validation_account_info, admin_mint_account_info)?;
    assert_mint_matching(voting_validation_account_info, voting_mint_account_info)?;
    assert_mint_matching(yes_voting_dump_account_info, yes_voting_mint_account_info)?;
    assert_mint_matching(no_voting_dump_account_info, no_voting_mint_account_info)?;
    assert_mint_matching(source_holding_account_info, source_mint_account_info)?;

    if source_holding_mint != timelock_config.governance_mint
        && source_holding_mint != timelock_config.council_mint
    {
        return Err(TimelockError::AccountsShouldMatch.into());
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

    let (authority_key, bump_seed) =
        Pubkey::find_program_address(&[timelock_program_info.key.as_ref()], program_id);
    if timelock_program_authority_info.key != &authority_key {
        return Err(TimelockError::InvalidTimelockAuthority.into());
    }
    let authority_signer_seeds = &[timelock_program_info.key.as_ref(), &[bump_seed]];

    spl_token_mint_to(TokenMintToParams {
        mint: admin_mint_account_info.clone(),
        destination: destination_admin_account_info.clone(),
        amount: 1,
        authority: timelock_program_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    spl_token_mint_to(TokenMintToParams {
        mint: signatory_mint_account_info.clone(),
        destination: destination_sig_account_info.clone(),
        amount: 1,
        authority: timelock_program_authority_info.clone(),
        authority_signer_seeds: authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;
    Ok(())
}
