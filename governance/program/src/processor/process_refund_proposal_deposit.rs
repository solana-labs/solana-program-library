//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};
use spl_governance_tools::account::dispose_account;

use crate::state::{
    proposal::get_proposal_data,
    proposal_deposit::get_proposal_deposit_data_for_proposal_and_deposit_payer,
};

/// Processes RefundProposalDeposit instruction
pub fn process_refund_proposal_deposit(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let proposal_info = next_account_info(account_info_iter)?; // 0

    let proposal_deposit_info = next_account_info(account_info_iter)?; // 1
    let proposal_deposit_payer_info = next_account_info(account_info_iter)?; // 2

    let proposal_data = get_proposal_data(program_id, proposal_info)?;

    proposal_data.assert_can_refund_proposal_deposit()?;

    // Assert we are disposing a deposit which belongs to the Proposal and the deposit payer
    let _proposal_deposit_data = get_proposal_deposit_data_for_proposal_and_deposit_payer(
        program_id,
        proposal_deposit_info,
        proposal_info.key,
        proposal_deposit_payer_info.key,
    )?;

    dispose_account(proposal_deposit_info, proposal_deposit_payer_info)?;

    Ok(())
}
