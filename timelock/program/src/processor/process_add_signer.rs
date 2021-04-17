//! Program state processor
use crate::{
    error::TimelockError,
    state::proposal::Proposal,
    state::proposal_state::ProposalState,
    utils::{
        assert_account_equiv, assert_draft, assert_initialized, assert_is_permissioned,
        assert_proper_signatory_mint, assert_token_program_is_correct, spl_token_mint_to,
        TokenMintToParams,
    },
    PROGRAM_AUTHORITY_SEED,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
};

/// Adds a signer
pub fn process_add_signer(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let new_signatory_account_info = next_account_info(account_info_iter)?;
    let signatory_mint_info = next_account_info(account_info_iter)?;
    let admin_account_info = next_account_info(account_info_iter)?;
    let admin_validation_account_info = next_account_info(account_info_iter)?;
    let proposal_state_account_info = next_account_info(account_info_iter)?;
    let proposal_account_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_authority_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;

    let mut proposal_state: ProposalState = assert_initialized(proposal_state_account_info)?;
    let proposal: Proposal = assert_initialized(proposal_account_info)?;
    assert_account_equiv(proposal_state_account_info, &proposal.state)?;
    assert_account_equiv(admin_validation_account_info, &proposal.admin_validation)?;
    assert_token_program_is_correct(&proposal, token_program_account_info)?;
    assert_proper_signatory_mint(&proposal, signatory_mint_info)?;
    assert_draft(&proposal_state)?;
    assert_is_permissioned(
        program_id,
        admin_account_info,
        admin_validation_account_info,
        proposal_account_info,
        token_program_account_info,
        transfer_authority_info,
        timelock_program_authority_info,
    )?;

    let mut seeds = vec![PROGRAM_AUTHORITY_SEED, proposal_account_info.key.as_ref()];

    let (authority_key, bump_seed) = Pubkey::find_program_address(&seeds[..], program_id);
    if timelock_program_authority_info.key != &authority_key {
        return Err(TimelockError::InvalidTimelockAuthority.into());
    }

    let bump = &[bump_seed];
    seeds.push(bump);
    let authority_signer_seeds = &seeds[..];

    spl_token_mint_to(TokenMintToParams {
        mint: signatory_mint_info.clone(),
        destination: new_signatory_account_info.clone(),
        amount: 1,
        authority: timelock_program_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_account_info.clone(),
    })?;

    proposal_state.total_signing_tokens_minted =
        match proposal_state.total_signing_tokens_minted.checked_add(1) {
            Some(val) => val,
            None => return Err(TimelockError::NumericalOverflow.into()),
        };

    ProposalState::pack(
        proposal_state,
        &mut proposal_state_account_info.data.borrow_mut(),
    )?;
    Ok(())
}
