//! On-chain program invoke helper to perform on-chain `transfer_checked` with
//! correct accounts

use {
    crate::{
        extension::{transfer_hook, StateWithExtensions},
        instruction,
        state::Mint,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, instruction::AccountMeta,
        program::invoke_signed, pubkey::Pubkey,
    },
    spl_transfer_hook_interface::onchain::add_extra_accounts_for_execute_cpi,
};

/// Helper to CPI into token-2022 on-chain, looking through the additional
/// account infos to create the proper instruction with the proper account infos
#[allow(clippy::too_many_arguments)]
pub fn invoke_transfer_checked<'a>(
    token_program_id: &Pubkey,
    source_info: AccountInfo<'a>,
    mint_info: AccountInfo<'a>,
    destination_info: AccountInfo<'a>,
    authority_info: AccountInfo<'a>,
    additional_accounts: &[AccountInfo<'a>],
    amount: u64,
    decimals: u8,
    seeds: &[&[&[u8]]],
) -> ProgramResult {
    let mut cpi_instruction = instruction::transfer_checked(
        token_program_id,
        source_info.key,
        mint_info.key,
        destination_info.key,
        authority_info.key,
        &[], // add them later, to avoid unnecessary clones
        amount,
        decimals,
    )?;

    let mut cpi_account_infos = vec![
        source_info.clone(),
        mint_info.clone(),
        destination_info.clone(),
        authority_info.clone(),
    ];

    // if it's a signer, it might be a multisig signer, throw it in!
    additional_accounts
        .iter()
        .filter(|ai| ai.is_signer)
        .for_each(|ai| {
            cpi_account_infos.push(ai.clone());
            cpi_instruction
                .accounts
                .push(AccountMeta::new_readonly(*ai.key, ai.is_signer));
        });

    // scope the borrowing to avoid a double-borrow during CPI
    {
        let mint_data = mint_info.try_borrow_data()?;
        let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;
        if let Some(program_id) = transfer_hook::get_program_id(&mint) {
            add_extra_accounts_for_execute_cpi(
                &mut cpi_instruction,
                &mut cpi_account_infos,
                &program_id,
                source_info,
                mint_info.clone(),
                destination_info,
                authority_info,
                amount,
                additional_accounts,
            )?;
        }
    }

    invoke_signed(&cpi_instruction, &cpi_account_infos, seeds)
}
