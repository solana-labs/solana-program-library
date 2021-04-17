//! Program state processor

use crate::utils::{assert_account_owner, assert_mint_authority, assert_mint_owner_program};
use crate::{
    error::GovernanceError,
    state::{
        enums::GovernanceAccountType,
        governance::Governance,
        proposal::Proposal,
        proposal_state::ProposalState,
        proposal_state::{DESC_SIZE, NAME_SIZE},
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

/// Create a new Proposal
pub fn process_init_proposal(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    name: [u8; NAME_SIZE],
    desc_link: [u8; DESC_SIZE],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let proposal_state_account_info = next_account_info(account_info_iter)?; //0
    let proposal_account_info = next_account_info(account_info_iter)?; //1
    let governance_account_info = next_account_info(account_info_iter)?; //2
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
    let governance_program_authority_info = next_account_info(account_info_iter)?; //17
    let token_program_info = next_account_info(account_info_iter)?; //18
    let rent_info = next_account_info(account_info_iter)?; //19
    let rent = &Rent::from_account_info(rent_info)?;

    let mut new_proposal_state: ProposalState = assert_uninitialized(proposal_state_account_info)?;
    let mut new_proposal: Proposal = assert_uninitialized(proposal_account_info)?;
    let mut governance: Governance = assert_initialized(governance_account_info)?;

    new_proposal.account_type = GovernanceAccountType::Proposal;
    new_proposal.config = *governance_account_info.key;
    new_proposal.token_program_id = *token_program_info.key;
    new_proposal.state = *proposal_state_account_info.key;
    new_proposal.admin_mint = *admin_mint_account_info.key;
    new_proposal.voting_mint = *voting_mint_account_info.key;
    new_proposal.yes_voting_mint = *yes_voting_mint_account_info.key;
    new_proposal.no_voting_mint = *no_voting_mint_account_info.key;
    new_proposal.source_mint = *source_mint_account_info.key;
    new_proposal.signatory_mint = *signatory_mint_account_info.key;
    new_proposal.source_holding = *source_holding_account_info.key;
    new_proposal.yes_voting_dump = *yes_voting_dump_account_info.key;
    new_proposal.no_voting_dump = *no_voting_dump_account_info.key;
    new_proposal.admin_validation = *admin_validation_account_info.key;
    new_proposal.voting_validation = *voting_validation_account_info.key;
    new_proposal.signatory_validation = *signatory_validation_account_info.key;

    new_proposal_state.account_type = GovernanceAccountType::ProposalState;
    new_proposal_state.proposal = *proposal_account_info.key;
    new_proposal_state.desc_link = desc_link;
    new_proposal_state.name = name;
    new_proposal_state.total_signing_tokens_minted = 1;
    new_proposal_state.number_of_executed_transactions = 0;
    new_proposal_state.number_of_transactions = 0;
    governance.count = match governance.count.checked_add(1) {
        Some(val) => val,
        None => return Err(GovernanceError::NumericalOverflow.into()),
    };

    assert_rent_exempt(rent, proposal_account_info)?;
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
        governance_program_authority_info.key,
    )?;
    assert_account_owner(
        admin_validation_account_info,
        governance_program_authority_info.key,
    )?;
    assert_account_owner(
        voting_validation_account_info,
        governance_program_authority_info.key,
    )?;
    assert_account_owner(
        yes_voting_dump_account_info,
        governance_program_authority_info.key,
    )?;
    assert_account_owner(
        no_voting_dump_account_info,
        governance_program_authority_info.key,
    )?;
    assert_account_owner(
        source_holding_account_info,
        governance_program_authority_info.key,
    )?;

    let source_mint_decimals = get_mint_decimals(source_mint_account_info)?;
    assert_mint_decimals(voting_mint_account_info, source_mint_decimals)?;
    assert_mint_decimals(yes_voting_mint_account_info, source_mint_decimals)?;
    assert_mint_decimals(no_voting_mint_account_info, source_mint_decimals)?;

    assert_mint_authority(
        signatory_mint_account_info,
        governance_program_authority_info.key,
    )?;
    assert_mint_authority(
        admin_mint_account_info,
        governance_program_authority_info.key,
    )?;
    assert_mint_authority(
        voting_mint_account_info,
        governance_program_authority_info.key,
    )?;
    assert_mint_authority(
        yes_voting_mint_account_info,
        governance_program_authority_info.key,
    )?;
    assert_mint_authority(
        no_voting_mint_account_info,
        governance_program_authority_info.key,
    )?;

    if source_holding_mint != governance.governance_mint {
        if let Some(council_mint) = governance.council_mint {
            if source_holding_mint != council_mint {
                return Err(GovernanceError::AccountsShouldMatch.into());
            }
        } else {
            return Err(GovernanceError::AccountsShouldMatch.into());
        }
    }

    Proposal::pack(new_proposal, &mut proposal_account_info.data.borrow_mut())?;
    ProposalState::pack(
        new_proposal_state,
        &mut proposal_state_account_info.data.borrow_mut(),
    )?;
    Governance::pack(governance, &mut governance_account_info.data.borrow_mut())?;

    let mut seeds = vec![PROGRAM_AUTHORITY_SEED, proposal_account_info.key.as_ref()];

    let (authority_key, bump_seed) = Pubkey::find_program_address(&seeds[..], program_id);
    if governance_program_authority_info.key != &authority_key {
        return Err(GovernanceError::InvalidGovernanceAuthority.into());
    }
    let bump = &[bump_seed];
    seeds.push(bump);
    let authority_signer_seeds = &seeds[..];

    spl_token_mint_to(TokenMintToParams {
        mint: admin_mint_account_info.clone(),
        destination: destination_admin_account_info.clone(),
        amount: 1,
        authority: governance_program_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;

    spl_token_mint_to(TokenMintToParams {
        mint: signatory_mint_account_info.clone(),
        destination: destination_sig_account_info.clone(),
        amount: 1,
        authority: governance_program_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_info.clone(),
    })?;
    Ok(())
}
