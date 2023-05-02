//! On-chain program invoke helper to perform on-chain `execute` with correct accounts

use {
    crate::{error::TransferHookError, get_extra_account_metas_address, instruction},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, program::invoke, pubkey::Pubkey,
    },
    spl_tlv_account_resolution::state::ExtraAccountMetas,
};

/// Helper to CPI into a transfer-hook program on-chain, looking through the
/// additional account infos to create the proper instruction
pub fn execute<'a>(
    program_id: &Pubkey,
    source_info: AccountInfo<'a>,
    mint_info: AccountInfo<'a>,
    destination_info: AccountInfo<'a>,
    authority_info: AccountInfo<'a>,
    additional_accounts: &[AccountInfo<'a>],
    amount: u64,
) -> ProgramResult {
    let validation_pubkey = get_extra_account_metas_address(mint_info.key, program_id);
    let validation_info = additional_accounts
        .iter()
        .find(|&x| *x.key == validation_pubkey)
        .ok_or(TransferHookError::IncorrectAccount)?;
    let mut cpi_instruction = instruction::execute(
        program_id,
        source_info.key,
        mint_info.key,
        destination_info.key,
        authority_info.key,
        &validation_pubkey,
        amount,
    );

    let mut cpi_account_infos = vec![
        source_info,
        mint_info,
        destination_info,
        authority_info,
        validation_info.clone(),
    ];
    ExtraAccountMetas::add_to_cpi_instruction_with_de_escalation::<instruction::ExecuteInstruction>(
        &mut cpi_instruction,
        &mut cpi_account_infos,
        &validation_info.try_borrow_data()?,
        additional_accounts,
    )?;
    invoke(&cpi_instruction, &cpi_account_infos)
}
