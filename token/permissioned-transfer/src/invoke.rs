//! On-chain program invoke helper to perform on-chain validation

use {
    crate::{
        error::PermissionedTransferError,
        get_extra_account_metas_address, instruction,
        state::ExtraAccountMetas,
        tlv::{TlvState, TlvStateBorrowed},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, program::invoke, pubkey::Pubkey,
    },
};

/// Helper to perform a validation on-chain, looking through the additional
/// account infos to create the proper instruction
pub fn validate<'a>(
    program_id: &Pubkey,
    source_info: AccountInfo<'a>,
    mint_info: AccountInfo<'a>,
    destination_info: AccountInfo<'a>,
    authority_info: AccountInfo<'a>,
    additional_accounts: &[AccountInfo<'a>],
    amount: u64,
) -> ProgramResult {
    // scope the borrowing to drop the account data before `invoke`
    let (validate_instruction, account_infos) = {
        let validation_pubkey = get_extra_account_metas_address(mint_info.key, program_id);
        let validation_info = additional_accounts
            .iter()
            .find(|&x| *x.key == validation_pubkey)
            .ok_or(PermissionedTransferError::IncorrectAccount)?;
        let validation_info_data = validation_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&validation_info_data)?;
        let bytes = state.get_bytes::<ExtraAccountMetas>()?;
        let extra_account_metas = ExtraAccountMetas::unpack(bytes)?;
        let additional_account_metas = extra_account_metas
            .data()
            .iter()
            .map(|&m| m.into())
            .collect::<Vec<_>>();

        let validate_instruction = instruction::validate(
            program_id,
            source_info.key,
            mint_info.key,
            destination_info.key,
            authority_info.key,
            &validation_pubkey,
            &additional_account_metas,
            amount,
        );

        let mut account_infos = vec![
            source_info,
            mint_info,
            destination_info,
            authority_info,
            validation_info.clone(),
        ];
        for account_meta in additional_account_metas {
            let account_info = additional_accounts
                .iter()
                .find(|&x| *x.key == account_meta.pubkey)
                .ok_or(PermissionedTransferError::IncorrectAccount)?
                .clone();
            account_infos.push(account_info);
        }
        (validate_instruction, account_infos)
    };
    invoke(&validate_instruction, &account_infos)
}
