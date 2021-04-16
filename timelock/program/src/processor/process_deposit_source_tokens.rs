//! Program state processor

use crate::{
    error::TimelockError,
    state::governance_voting_record::{GovernanceVotingRecord, GOVERNANCE_VOTING_RECORD_VERSION},
    state::timelock_set::TimelockSet,
    utils::{
        assert_account_equiv, assert_initialized, assert_token_program_is_correct,
        spl_token_mint_to, spl_token_transfer, TokenMintToParams, TokenTransferParams,
    },
    AUTHORITY_SEED_PROPOSAL, AUTHORITY_SEED_PROPOSAL_VOTE,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
};
use spl_token::state::Account;

/// Deposit source tokens
pub fn process_deposit_source_tokens(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    voting_token_amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let voting_record_account_info = next_account_info(account_info_iter)?;
    let voting_account_info = next_account_info(account_info_iter)?;
    let user_holding_account_info = next_account_info(account_info_iter)?;
    let source_holding_account_info = next_account_info(account_info_iter)?;
    let voting_mint_account_info = next_account_info(account_info_iter)?;
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_authority_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;

    let timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    assert_token_program_is_correct(&timelock_set, token_program_account_info)?;

    assert_account_equiv(source_holding_account_info, &timelock_set.source_holding)?;
    assert_account_equiv(voting_mint_account_info, &timelock_set.voting_mint)?;

    let mut seeds = vec![
        AUTHORITY_SEED_PROPOSAL,
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
        mint: voting_mint_account_info.clone(),
        destination: voting_account_info.clone(),
        amount: voting_token_amount,
        authority: timelock_program_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_account_info.clone(),
    })?;

    spl_token_transfer(TokenTransferParams {
        source: user_holding_account_info.clone(),
        destination: source_holding_account_info.clone(),
        amount: voting_token_amount,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_account_info.clone(),
    })?;

    let (voting_record_key, _) = Pubkey::find_program_address(
        &[
            AUTHORITY_SEED_PROPOSAL_VOTE,
            program_id.as_ref(),
            timelock_set_account_info.key.as_ref(),
            voting_account_info.key.as_ref(),
        ],
        program_id,
    );
    if voting_record_account_info.key != &voting_record_key {
        return Err(TimelockError::InvalidGovernanceVotingRecord.into());
    }

    let mut voting_record: GovernanceVotingRecord =
        GovernanceVotingRecord::unpack_unchecked(&voting_record_account_info.data.borrow())?;
    if !voting_record.is_initialized() {
        let voting_account: Account = assert_initialized(voting_account_info)?;
        voting_record.proposal = *timelock_set_account_info.key;
        voting_record.owner = voting_account.owner;
        voting_record.version = GOVERNANCE_VOTING_RECORD_VERSION;
        voting_record.undecided_count = voting_token_amount;
        voting_record.yes_count = 0;
        voting_record.no_count = 0;
    } else {
        voting_record.undecided_count = match voting_record
            .undecided_count
            .checked_add(voting_token_amount)
        {
            Some(val) => val,
            None => return Err(TimelockError::NumericalOverflow.into()),
        };
    }
    GovernanceVotingRecord::pack(
        voting_record,
        &mut voting_record_account_info.data.borrow_mut(),
    )?;
    Ok(())
}
