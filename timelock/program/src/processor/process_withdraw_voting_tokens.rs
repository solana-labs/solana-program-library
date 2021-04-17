//! Program state processor

use crate::{
    error::TimelockError,
    state::governance_voting_record::GovernanceVotingRecord,
    state::{enums::ProposalStateStatus, proposal::Proposal, proposal_state::ProposalState},
    utils::{
        assert_account_equiv, assert_initialized, assert_token_program_is_correct, spl_token_burn,
        spl_token_transfer, TokenBurnParams, TokenTransferParams,
    },
    PROGRAM_AUTHORITY_SEED,
};

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    program_pack::Pack,
    pubkey::Pubkey,
};
use spl_token::state::Account;

/// Withdraw voting tokens
pub fn process_withdraw_voting_tokens(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    voting_token_amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let voting_record_account_info = next_account_info(account_info_iter)?;
    let voting_account_info = next_account_info(account_info_iter)?;
    let yes_voting_account_info = next_account_info(account_info_iter)?;
    let no_voting_account_info = next_account_info(account_info_iter)?;
    let user_account_info = next_account_info(account_info_iter)?;
    let source_holding_account_info = next_account_info(account_info_iter)?;
    let yes_voting_dump_account_info = next_account_info(account_info_iter)?;
    let no_voting_dump_account_info = next_account_info(account_info_iter)?;
    let voting_mint_account_info = next_account_info(account_info_iter)?;
    let yes_voting_mint_account_info = next_account_info(account_info_iter)?;
    let no_voting_mint_account_info = next_account_info(account_info_iter)?;

    let proposal_state_account_info = next_account_info(account_info_iter)?;
    let proposal_account_info = next_account_info(account_info_iter)?;

    let transfer_authority_info = next_account_info(account_info_iter)?;
    let governance_program_authority_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;

    let proposal_state: ProposalState = assert_initialized(proposal_state_account_info)?;
    let proposal: Proposal = assert_initialized(proposal_account_info)?;
    assert_token_program_is_correct(&proposal, token_program_account_info)?;
    // Using assert_account_equiv not workable here due to cost of stack size on this method.

    assert_account_equiv(proposal_state_account_info, &proposal.state)?;
    assert_account_equiv(voting_mint_account_info, &proposal.voting_mint)?;
    assert_account_equiv(yes_voting_mint_account_info, &proposal.yes_voting_mint)?;
    assert_account_equiv(no_voting_mint_account_info, &proposal.no_voting_mint)?;
    assert_account_equiv(yes_voting_dump_account_info, &proposal.yes_voting_dump)?;
    assert_account_equiv(no_voting_dump_account_info, &proposal.no_voting_dump)?;
    assert_account_equiv(source_holding_account_info, &proposal.source_holding)?;

    let voting_account: Account = assert_initialized(voting_account_info)?;
    let yes_voting_account: Account = assert_initialized(yes_voting_account_info)?;
    let no_voting_account: Account = assert_initialized(no_voting_account_info)?;

    let mut seeds = vec![PROGRAM_AUTHORITY_SEED, proposal_account_info.key.as_ref()];

    let (authority_key, bump_seed) = Pubkey::find_program_address(&seeds[..], program_id);
    if governance_program_authority_info.key != &authority_key {
        return Err(TimelockError::InvalidTimelockAuthority.into());
    }
    let bump = &[bump_seed];
    seeds.push(bump);
    let authority_signer_seeds = &seeds[..];

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
        return Err(TimelockError::InvalidGovernanceVotingRecord.into());
    }

    let mut voting_record: GovernanceVotingRecord =
        GovernanceVotingRecord::unpack_unchecked(&voting_record_account_info.data.borrow())?;

    // prefer voting account first, then yes, then no. Invariants we know are
    // voting_token_amount <= voting + yes + no
    // voting_token_amount <= voting
    // voting_token_amount <= yes
    // voting_token_amount <= no
    // because at best they dumped 100 in and that 100 is mixed between all 3 or all in one.

    let mut total_possible: u64;

    total_possible = match voting_account.amount.checked_add(yes_voting_account.amount) {
        Some(val) => val,
        None => return Err(TimelockError::NumericalOverflow.into()),
    };
    total_possible = match total_possible.checked_add(no_voting_account.amount) {
        Some(val) => val,
        None => return Err(TimelockError::NumericalOverflow.into()),
    };

    let mut voting_fuel_tank = voting_token_amount;
    if voting_token_amount > total_possible {
        return Err(TimelockError::TokenAmountAboveGivenAmount.into());
    }

    if voting_account.amount > 0 {
        let amount_to_burn: u64;
        if voting_account.amount < voting_fuel_tank {
            amount_to_burn = voting_account.amount;
            voting_fuel_tank = match voting_fuel_tank.checked_sub(amount_to_burn) {
                Some(val) => val,
                None => return Err(TimelockError::NumericalOverflow.into()),
            };
        } else {
            amount_to_burn = voting_fuel_tank;
            voting_fuel_tank = 0;
        }
        if amount_to_burn > 0 {
            spl_token_burn(TokenBurnParams {
                mint: voting_mint_account_info.clone(),
                amount: amount_to_burn,
                authority: transfer_authority_info.clone(),
                authority_signer_seeds,
                token_program: token_program_account_info.clone(),
                source: voting_account_info.clone(),
            })?;
            voting_record.undecided_count =
                match voting_record.undecided_count.checked_sub(amount_to_burn) {
                    Some(val) => val,
                    None => return Err(TimelockError::NumericalOverflow.into()),
                };
        }
    }

    if yes_voting_account.amount > 0 {
        let amount_to_transfer: u64;
        if yes_voting_account.amount < voting_fuel_tank {
            amount_to_transfer = yes_voting_account.amount;
            voting_fuel_tank = match voting_fuel_tank.checked_sub(amount_to_transfer) {
                Some(val) => val,
                None => return Err(TimelockError::NumericalOverflow.into()),
            };
        } else {
            amount_to_transfer = voting_fuel_tank;
            voting_fuel_tank = 0;
        }

        if amount_to_transfer > 0 {
            if proposal_state.status == ProposalStateStatus::Voting {
                spl_token_burn(TokenBurnParams {
                    mint: yes_voting_mint_account_info.clone(),
                    amount: amount_to_transfer,
                    authority: transfer_authority_info.clone(),
                    authority_signer_seeds,
                    token_program: token_program_account_info.clone(),
                    source: yes_voting_account_info.clone(),
                })?;
            } else {
                spl_token_transfer(TokenTransferParams {
                    source: yes_voting_account_info.clone(),
                    destination: yes_voting_dump_account_info.clone(),
                    amount: amount_to_transfer,
                    authority: transfer_authority_info.clone(),
                    authority_signer_seeds,
                    token_program: token_program_account_info.clone(),
                })?;
            }
            voting_record.yes_count = match voting_record.yes_count.checked_sub(amount_to_transfer)
            {
                Some(val) => val,
                None => return Err(TimelockError::NumericalOverflow.into()),
            };
        }
    }

    if no_voting_account.amount > 0 && voting_fuel_tank > 0 {
        // whatever is left, no account gets by default
        if proposal_state.status == ProposalStateStatus::Voting {
            spl_token_burn(TokenBurnParams {
                mint: no_voting_mint_account_info.clone(),
                amount: voting_fuel_tank,
                authority: transfer_authority_info.clone(),
                authority_signer_seeds,
                token_program: token_program_account_info.clone(),
                source: no_voting_account_info.clone(),
            })?;
        } else {
            spl_token_transfer(TokenTransferParams {
                source: no_voting_account_info.clone(),
                destination: no_voting_dump_account_info.clone(),
                amount: voting_fuel_tank,
                authority: transfer_authority_info.clone(),
                authority_signer_seeds,
                token_program: token_program_account_info.clone(),
            })?;
        }
        voting_record.no_count = match voting_record.no_count.checked_sub(voting_fuel_tank) {
            Some(val) => val,
            None => return Err(TimelockError::NumericalOverflow.into()),
        };
    }

    spl_token_transfer(TokenTransferParams {
        source: source_holding_account_info.clone(),
        destination: user_account_info.clone(),
        amount: voting_token_amount,
        authority: governance_program_authority_info.clone(),
        authority_signer_seeds,
        token_program: token_program_account_info.clone(),
    })?;

    GovernanceVotingRecord::pack(
        voting_record,
        &mut voting_record_account_info.data.borrow_mut(),
    )?;
    Ok(())
}
