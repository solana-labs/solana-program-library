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
    spl_transfer_hook_interface::{
        error::TransferHookError, get_extra_account_metas_address,
        onchain::add_cpi_accounts_for_execute,
    },
};

/// Helper to CPI into token-2022 on-chain, looking through the additional
/// account infos to create the proper instruction with the proper account
/// infos.
///
/// Note that this onchain helper will build a new `Execute` instruction,
/// resolve the extra account metas, and then add them to the transfer
/// instruction. This is because the extra account metas are configured
/// specifically for the `Execute` instruction, which requires five accounts
/// (source, mint, destination, authority, and validation state), wheras the
/// transfer instruction only requires four (source, mint, destination, and
/// authority) in addition to `n` number of multisig authorities.
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
    let mut transfer_cpi_ix = instruction::transfer_checked(
        token_program_id,
        source_info.key,
        mint_info.key,
        destination_info.key,
        authority_info.key,
        &[], // add them later, to avoid unnecessary clones
        amount,
        decimals,
    )?;

    let mut transfer_cpi_account_infos = vec![
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
            transfer_cpi_account_infos.push(ai.clone());
            transfer_cpi_ix
                .accounts
                .push(AccountMeta::new_readonly(*ai.key, ai.is_signer));
        });

    if token_program_id == &crate::id() {
        let mint_data = mint_info.try_borrow_data()?;
        let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;
        if let Some(program_id) = transfer_hook::get_program_id(&mint) {
            // Convert the transfer instruction into an `Execute` instruction,
            // then resolve the extra account metas as configured in the validation
            // account data, then finally add the extra account metas to the original
            // transfer instruction.
            let validation_pubkey = get_extra_account_metas_address(mint_info.key, &program_id);
            let validation_info = additional_accounts
                .iter()
                .find(|&x| *x.key == validation_pubkey)
                .ok_or(TransferHookError::IncorrectAccount)?;
            transfer_cpi_account_infos.push(validation_info.clone());

            let mut execute_ix = spl_transfer_hook_interface::instruction::execute(
                &program_id,
                source_info.key,
                mint_info.key,
                destination_info.key,
                authority_info.key,
                &validation_pubkey,
                amount,
            );

            add_cpi_accounts_for_execute(
                &mut execute_ix,
                &mut transfer_cpi_account_infos,
                mint_info.key,
                &program_id,
                additional_accounts,
            )?;

            transfer_cpi_ix
                .accounts
                .extend_from_slice(&execute_ix.accounts[5..]);
        }
    }

    invoke_signed(&transfer_cpi_ix, &transfer_cpi_account_infos, seeds)
}
