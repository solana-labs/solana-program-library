//! Program state processor
use crate::{
    error::GovernanceError,
    state::{
        enums::ProposalStateStatus, governance::Governance,
        governance_voting_record::GovernanceVotingRecord, proposal::Proposal,
        proposal_state::ProposalState,
    },
    utils::{
        assert_account_equiv, assert_initialized, assert_voting, get_mint_supply, spl_token_burn,
        spl_token_mint_to, TokenBurnParams, TokenMintToParams,
    },
    PROGRAM_AUTHORITY_SEED,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use spl_token::state::Account;

/// Vote on the timelock
pub fn process_vote(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    yes_voting_token_amount: u64,
    no_voting_token_amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let voting_record_account_info = next_account_info(account_info_iter)?; // 0
    let proposal_state_account_info = next_account_info(account_info_iter)?; // 1
    let voting_account_info = next_account_info(account_info_iter)?; //2
    let yes_voting_account_info = next_account_info(account_info_iter)?; //3
    let no_voting_account_info = next_account_info(account_info_iter)?; //4
    let voting_mint_account_info = next_account_info(account_info_iter)?; //5
    let yes_voting_mint_account_info = next_account_info(account_info_iter)?; //6
    let no_voting_mint_account_info = next_account_info(account_info_iter)?; //7
    let source_mint_account_info = next_account_info(account_info_iter)?; //8
    let proposal_account_info = next_account_info(account_info_iter)?; //9
    let governance_account_info = next_account_info(account_info_iter)?; //10
    let transfer_authority_info = next_account_info(account_info_iter)?; //11
    let governance_program_authority_info = next_account_info(account_info_iter)?; //12
    let token_program_account_info = next_account_info(account_info_iter)?; //13
    let clock_info = next_account_info(account_info_iter)?; //14

    let clock = Clock::from_account_info(clock_info)?;
    let mut proposal_state: ProposalState = assert_initialized(proposal_state_account_info)?;
    let proposal: Proposal = assert_initialized(proposal_account_info)?;
    let governance: Governance = assert_initialized(governance_account_info)?;

    assert_account_equiv(voting_mint_account_info, &proposal.voting_mint)?;
    assert_account_equiv(yes_voting_mint_account_info, &proposal.yes_voting_mint)?;
    assert_account_equiv(no_voting_mint_account_info, &proposal.no_voting_mint)?;
    assert_account_equiv(governance_account_info, &proposal.config)?;
    assert_account_equiv(proposal_state_account_info, &proposal.state)?;
    assert_account_equiv(source_mint_account_info, &proposal.source_mint)?;

    assert_voting(&proposal_state)?;

    let mut seeds = vec![PROGRAM_AUTHORITY_SEED, proposal_account_info.key.as_ref()];

    let (authority_key, bump_seed) = Pubkey::find_program_address(&seeds[..], program_id);
    if governance_program_authority_info.key != &authority_key {
        return Err(GovernanceError::InvalidTimelockAuthority.into());
    }
    let bump = &[bump_seed];
    seeds.push(bump);
    let authority_signer_seeds = &seeds[..];

    // We don't initialize the mints because it's too expensive on the stack size.
    let source_mint_supply: u64 = get_mint_supply(source_mint_account_info)?;
    let yes_mint_supply: u64 = get_mint_supply(yes_voting_mint_account_info)?;

    let total_ever_existed = source_mint_supply;

    let mut now_remaining_in_no_column = source_mint_supply
        .checked_sub(yes_voting_token_amount)
        .ok_or(GovernanceError::NumericalOverflow)?;

    now_remaining_in_no_column = now_remaining_in_no_column
        .checked_sub(yes_mint_supply)
        .ok_or(GovernanceError::NumericalOverflow)?;

    let starting_vote_acct: Account = assert_initialized(voting_account_info)?;
    let yes_vote_acct: Account = assert_initialized(yes_voting_account_info)?;
    let no_vote_acct: Account = assert_initialized(no_voting_account_info)?;

    // The act of voting proves you are able to vote. No need to assert permission here.
    spl_token_burn(TokenBurnParams {
        mint: voting_mint_account_info.clone(),
        amount: yes_voting_token_amount + no_voting_token_amount,
        authority: transfer_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_account_info.clone(),
        source: voting_account_info.clone(),
    })?;

    spl_token_mint_to(TokenMintToParams {
        mint: yes_voting_mint_account_info.clone(),
        destination: yes_voting_account_info.clone(),
        amount: yes_voting_token_amount,
        authority: governance_program_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_account_info.clone(),
    })?;

    spl_token_mint_to(TokenMintToParams {
        mint: no_voting_mint_account_info.clone(),
        destination: no_voting_account_info.clone(),
        amount: no_voting_token_amount,
        authority: governance_program_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_account_info.clone(),
    })?;

    let tipped: bool = now_remaining_in_no_column == 0
        || ((1.0 - now_remaining_in_no_column as f64 / total_ever_existed as f64) * 100.0
            >= governance.vote_threshold as f64);

    let elapsed = match clock.slot.checked_sub(proposal_state.voting_began_at) {
        Some(val) => val,
        None => return Err(GovernanceError::NumericalOverflow.into()),
    };
    let too_long = elapsed > governance.time_limit;

    if tipped || too_long {
        if tipped {
            proposal_state.status = ProposalStateStatus::Executing;
        } else {
            proposal_state.status = ProposalStateStatus::Defeated;
        }
        proposal_state.voting_ended_at = clock.slot;

        ProposalState::pack(
            proposal_state,
            &mut proposal_state_account_info.data.borrow_mut(),
        )?;
    }
    let (voting_record_key, _) = Pubkey::find_program_address(
        &[
            PROGRAM_AUTHORITY_SEED,
            program_id.as_ref(),
            proposal_account_info.key.as_ref(),
            voting_account_info.key.as_ref(),
        ],
        program_id,
    );
    if voting_record_account_info.key != &voting_record_key {
        return Err(GovernanceError::InvalidGovernanceVotingRecord.into());
    }

    let mut voting_record: GovernanceVotingRecord =
        GovernanceVotingRecord::unpack_unchecked(&voting_record_account_info.data.borrow())?;

    voting_record.yes_count = match yes_vote_acct.amount.checked_add(yes_voting_token_amount) {
        Some(val) => val,
        None => return Err(GovernanceError::NumericalOverflow.into()),
    };
    voting_record.no_count = match no_vote_acct.amount.checked_add(no_voting_token_amount) {
        Some(val) => val,
        None => return Err(GovernanceError::NumericalOverflow.into()),
    };
    let total_change = match yes_voting_token_amount.checked_add(no_voting_token_amount) {
        Some(val) => val,
        None => return Err(GovernanceError::NumericalOverflow.into()),
    };
    voting_record.undecided_count = match starting_vote_acct.amount.checked_sub(total_change) {
        Some(val) => val,
        None => return Err(GovernanceError::NumericalOverflow.into()),
    };
    GovernanceVotingRecord::pack(
        voting_record,
        &mut voting_record_account_info.data.borrow_mut(),
    )?;

    Ok(())
}
