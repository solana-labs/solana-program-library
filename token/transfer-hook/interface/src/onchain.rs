//! On-chain program invoke helper to perform on-chain `execute` with correct
//! accounts

use {
    crate::{error::TransferHookError, get_extra_account_metas_address, instruction},
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        program::invoke,
        pubkey::Pubkey,
    },
    spl_tlv_account_resolution::state::ExtraAccountMetaList,
};
/// Helper to CPI into a transfer-hook program on-chain, looking through the
/// additional account infos to create the proper instruction
pub fn invoke_execute<'a>(
    program_id: &Pubkey,
    source_info: AccountInfo<'a>,
    mint_info: AccountInfo<'a>,
    destination_info: AccountInfo<'a>,
    authority_info: AccountInfo<'a>,
    additional_accounts: &[AccountInfo<'a>],
    amount: u64,
) -> ProgramResult {
    let mut cpi_instruction = instruction::execute(
        program_id,
        source_info.key,
        mint_info.key,
        destination_info.key,
        authority_info.key,
        amount,
    );

    let validation_pubkey = get_extra_account_metas_address(mint_info.key, program_id);

    let mut cpi_account_infos = vec![source_info, mint_info, destination_info, authority_info];

    if let Some(validation_info) = additional_accounts
        .iter()
        .find(|&x| *x.key == validation_pubkey)
    {
        cpi_instruction
            .accounts
            .push(AccountMeta::new_readonly(validation_pubkey, false));
        cpi_account_infos.push(validation_info.clone());

        ExtraAccountMetaList::add_to_cpi_instruction::<instruction::ExecuteInstruction>(
            &mut cpi_instruction,
            &mut cpi_account_infos,
            &validation_info.try_borrow_data()?,
            additional_accounts,
        )?;
    }

    invoke(&cpi_instruction, &cpi_account_infos)
}

/// Helper to add accounts required for an `ExecuteInstruction` on-chain,
/// looking through the additional account infos to add the proper accounts.
///
/// Note this helper is designed to add the extra accounts that will be
/// required for a CPI to a transfer hook program. However, the instruction
/// being provided to this helper is for the program that will CPI to the
/// transfer hook program. Because of this, we must resolve the extra accounts
/// for the `ExecuteInstruction` CPI, then add those extra resolved accounts to
/// the provided instruction.
#[allow(clippy::too_many_arguments)]
pub fn add_extra_accounts_for_execute_cpi<'a>(
    cpi_instruction: &mut Instruction,
    cpi_account_infos: &mut Vec<AccountInfo<'a>>,
    program_id: &Pubkey,
    source_info: AccountInfo<'a>,
    mint_info: AccountInfo<'a>,
    destination_info: AccountInfo<'a>,
    authority_info: AccountInfo<'a>,
    amount: u64,
    additional_accounts: &[AccountInfo<'a>],
) -> ProgramResult {
    let validate_state_pubkey = get_extra_account_metas_address(mint_info.key, program_id);

    let program_info = additional_accounts
        .iter()
        .find(|&x| x.key == program_id)
        .ok_or(TransferHookError::IncorrectAccount)?;

    if let Some(validate_state_info) = additional_accounts
        .iter()
        .find(|&x| *x.key == validate_state_pubkey)
    {
        let mut execute_instruction = instruction::execute(
            program_id,
            source_info.key,
            mint_info.key,
            destination_info.key,
            authority_info.key,
            amount,
        );
        execute_instruction
            .accounts
            .push(AccountMeta::new_readonly(validate_state_pubkey, false));
        let mut execute_account_infos = vec![
            source_info,
            mint_info,
            destination_info,
            authority_info,
            validate_state_info.clone(),
        ];

        ExtraAccountMetaList::add_to_cpi_instruction::<instruction::ExecuteInstruction>(
            &mut execute_instruction,
            &mut execute_account_infos,
            &validate_state_info.try_borrow_data()?,
            additional_accounts,
        )?;

        // Add only the extra accounts resolved from the validation state
        cpi_instruction
            .accounts
            .extend_from_slice(&execute_instruction.accounts[5..]);
        cpi_account_infos.extend_from_slice(&execute_account_infos[5..]);

        // Add the validation state account
        cpi_instruction
            .accounts
            .push(AccountMeta::new_readonly(validate_state_pubkey, false));
        cpi_account_infos.push(validate_state_info.clone());
    }

    // Add the program id
    cpi_instruction
        .accounts
        .push(AccountMeta::new_readonly(*program_id, false));
    cpi_account_infos.push(program_info.clone());

    Ok(())
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::instruction::ExecuteInstruction,
        solana_program::{bpf_loader_upgradeable, system_program},
        spl_tlv_account_resolution::{
            account::ExtraAccountMeta, error::AccountResolutionError, seeds::Seed,
        },
    };

    const EXTRA_META_1: Pubkey = Pubkey::new_from_array([2u8; 32]);
    const EXTRA_META_2: Pubkey = Pubkey::new_from_array([3u8; 32]);

    fn setup_validation_data() -> Vec<u8> {
        let extra_metas = vec![
            ExtraAccountMeta::new_with_pubkey(&EXTRA_META_1, true, false).unwrap(),
            ExtraAccountMeta::new_with_pubkey(&EXTRA_META_2, true, false).unwrap(),
            ExtraAccountMeta::new_with_seeds(
                &[
                    Seed::AccountKey { index: 0 }, // source
                    Seed::AccountKey { index: 2 }, // destination
                    Seed::AccountKey { index: 4 }, // validation state
                ],
                false,
                true,
            )
            .unwrap(),
            ExtraAccountMeta::new_with_seeds(
                &[
                    Seed::InstructionData {
                        index: 8,
                        length: 8,
                    }, // amount
                    Seed::AccountKey { index: 2 }, // destination
                    Seed::AccountKey { index: 5 }, // extra meta 1
                    Seed::AccountKey { index: 7 }, // extra meta 3 (PDA)
                ],
                false,
                true,
            )
            .unwrap(),
        ];
        let account_size = ExtraAccountMetaList::size_of(extra_metas.len()).unwrap();
        let mut data = vec![0u8; account_size];
        ExtraAccountMetaList::init::<ExecuteInstruction>(&mut data, &extra_metas).unwrap();
        data
    }

    #[test]
    fn test_add_extra_accounts_for_execute_cpi() {
        let spl_token_2022_program_id = Pubkey::new_unique(); // Mock
        let transfer_hook_program_id = Pubkey::new_unique();

        let amount = 100u64;

        let source_pubkey = Pubkey::new_unique();
        let mut source_data = vec![0; 165]; // Mock
        let mut source_lamports = 0; // Mock
        let source_account_info = AccountInfo::new(
            &source_pubkey,
            false,
            true,
            &mut source_lamports,
            &mut source_data,
            &spl_token_2022_program_id,
            false,
            0,
        );

        let mint_pubkey = Pubkey::new_unique();
        let mut mint_data = vec![0; 165]; // Mock
        let mut mint_lamports = 0; // Mock
        let mint_account_info = AccountInfo::new(
            &mint_pubkey,
            false,
            true,
            &mut mint_lamports,
            &mut mint_data,
            &spl_token_2022_program_id,
            false,
            0,
        );

        let destination_pubkey = Pubkey::new_unique();
        let mut destination_data = vec![0; 165]; // Mock
        let mut destination_lamports = 0; // Mock
        let destination_account_info = AccountInfo::new(
            &destination_pubkey,
            false,
            true,
            &mut destination_lamports,
            &mut destination_data,
            &spl_token_2022_program_id,
            false,
            0,
        );

        let authority_pubkey = Pubkey::new_unique();
        let mut authority_data = vec![]; // Mock
        let mut authority_lamports = 0; // Mock
        let authority_account_info = AccountInfo::new(
            &authority_pubkey,
            false,
            true,
            &mut authority_lamports,
            &mut authority_data,
            &system_program::ID,
            false,
            0,
        );

        let validate_state_pubkey =
            get_extra_account_metas_address(&mint_pubkey, &transfer_hook_program_id);

        let extra_meta_1_pubkey = EXTRA_META_1;
        let mut extra_meta_1_data = vec![]; // Mock
        let mut extra_meta_1_lamports = 0; // Mock
        let extra_meta_1_account_info = AccountInfo::new(
            &extra_meta_1_pubkey,
            true,
            false,
            &mut extra_meta_1_lamports,
            &mut extra_meta_1_data,
            &system_program::ID,
            false,
            0,
        );

        let extra_meta_2_pubkey = EXTRA_META_2;
        let mut extra_meta_2_data = vec![]; // Mock
        let mut extra_meta_2_lamports = 0; // Mock
        let extra_meta_2_account_info = AccountInfo::new(
            &extra_meta_2_pubkey,
            true,
            false,
            &mut extra_meta_2_lamports,
            &mut extra_meta_2_data,
            &system_program::ID,
            false,
            0,
        );

        let extra_meta_3_pubkey = Pubkey::find_program_address(
            &[
                &source_pubkey.to_bytes(),
                &destination_pubkey.to_bytes(),
                &validate_state_pubkey.to_bytes(),
            ],
            &transfer_hook_program_id,
        )
        .0;
        let mut extra_meta_3_data = vec![]; // Mock
        let mut extra_meta_3_lamports = 0; // Mock
        let extra_meta_3_account_info = AccountInfo::new(
            &extra_meta_3_pubkey,
            false,
            true,
            &mut extra_meta_3_lamports,
            &mut extra_meta_3_data,
            &transfer_hook_program_id,
            false,
            0,
        );

        let extra_meta_4_pubkey = Pubkey::find_program_address(
            &[
                &amount.to_le_bytes(),
                &destination_pubkey.to_bytes(),
                &extra_meta_1_pubkey.to_bytes(),
                &extra_meta_3_pubkey.to_bytes(),
            ],
            &transfer_hook_program_id,
        )
        .0;
        let mut extra_meta_4_data = vec![]; // Mock
        let mut extra_meta_4_lamports = 0; // Mock
        let extra_meta_4_account_info = AccountInfo::new(
            &extra_meta_4_pubkey,
            false,
            true,
            &mut extra_meta_4_lamports,
            &mut extra_meta_4_data,
            &transfer_hook_program_id,
            false,
            0,
        );

        let mut validate_state_data = setup_validation_data();
        let mut validate_state_lamports = 0; // Mock
        let validate_state_account_info = AccountInfo::new(
            &validate_state_pubkey,
            false,
            true,
            &mut validate_state_lamports,
            &mut validate_state_data,
            &transfer_hook_program_id,
            false,
            0,
        );

        let mut transfer_hook_program_data = vec![]; // Mock
        let mut transfer_hook_program_lamports = 0; // Mock
        let transfer_hook_program_account_info = AccountInfo::new(
            &transfer_hook_program_id,
            false,
            true,
            &mut transfer_hook_program_lamports,
            &mut transfer_hook_program_data,
            &bpf_loader_upgradeable::ID,
            false,
            0,
        );

        let mut cpi_instruction = Instruction::new_with_bytes(
            spl_token_2022_program_id,
            &[],
            vec![
                AccountMeta::new(source_pubkey, false),
                AccountMeta::new_readonly(mint_pubkey, false),
                AccountMeta::new(destination_pubkey, false),
                AccountMeta::new_readonly(authority_pubkey, true),
            ],
        );
        let mut cpi_account_infos = vec![
            source_account_info.clone(),
            mint_account_info.clone(),
            destination_account_info.clone(),
            authority_account_info.clone(),
        ];
        let additional_account_infos = vec![
            extra_meta_1_account_info.clone(),
            extra_meta_2_account_info.clone(),
            extra_meta_3_account_info.clone(),
            extra_meta_4_account_info.clone(),
            transfer_hook_program_account_info.clone(),
            validate_state_account_info.clone(),
        ];

        // Allow missing validation info from additional account infos
        {
            let additional_account_infos_missing_infos = vec![
                extra_meta_1_account_info.clone(),
                extra_meta_2_account_info.clone(),
                extra_meta_3_account_info.clone(),
                extra_meta_4_account_info.clone(),
                // validate state missing
                transfer_hook_program_account_info.clone(),
            ];
            let mut cpi_instruction = cpi_instruction.clone();
            let mut cpi_account_infos = cpi_account_infos.clone();
            add_extra_accounts_for_execute_cpi(
                &mut cpi_instruction,
                &mut cpi_account_infos,
                &transfer_hook_program_id,
                source_account_info.clone(),
                mint_account_info.clone(),
                destination_account_info.clone(),
                authority_account_info.clone(),
                amount,
                &additional_account_infos_missing_infos,
            )
            .unwrap();
            let check_metas = [
                AccountMeta::new(source_pubkey, false),
                AccountMeta::new_readonly(mint_pubkey, false),
                AccountMeta::new(destination_pubkey, false),
                AccountMeta::new_readonly(authority_pubkey, true),
                AccountMeta::new_readonly(transfer_hook_program_id, false),
            ];

            let check_account_infos = vec![
                source_account_info.clone(),
                mint_account_info.clone(),
                destination_account_info.clone(),
                authority_account_info.clone(),
                transfer_hook_program_account_info.clone(),
            ];

            assert_eq!(cpi_instruction.accounts, check_metas);
            for (a, b) in std::iter::zip(cpi_account_infos, check_account_infos) {
                assert_eq!(a.key, b.key);
                assert_eq!(a.is_signer, b.is_signer);
                assert_eq!(a.is_writable, b.is_writable);
            }
        }

        // Fail missing program info from additional account infos
        let additional_account_infos_missing_infos = vec![
            extra_meta_1_account_info.clone(),
            extra_meta_2_account_info.clone(),
            extra_meta_3_account_info.clone(),
            extra_meta_4_account_info.clone(),
            validate_state_account_info.clone(),
            // transfer hook program missing
        ];
        assert_eq!(
            add_extra_accounts_for_execute_cpi(
                &mut cpi_instruction,
                &mut cpi_account_infos,
                &transfer_hook_program_id,
                source_account_info.clone(),
                mint_account_info.clone(),
                destination_account_info.clone(),
                authority_account_info.clone(),
                amount,
                &additional_account_infos_missing_infos, // Missing account info
            )
            .unwrap_err(),
            TransferHookError::IncorrectAccount.into()
        );

        // Fail missing extra meta info from additional account infos
        let additional_account_infos_missing_infos = vec![
            extra_meta_1_account_info.clone(),
            extra_meta_2_account_info.clone(),
            // extra meta 3 missing
            extra_meta_4_account_info.clone(),
            validate_state_account_info.clone(),
            transfer_hook_program_account_info.clone(),
        ];
        assert_eq!(
            add_extra_accounts_for_execute_cpi(
                &mut cpi_instruction,
                &mut cpi_account_infos,
                &transfer_hook_program_id,
                source_account_info.clone(),
                mint_account_info.clone(),
                destination_account_info.clone(),
                authority_account_info.clone(),
                amount,
                &additional_account_infos_missing_infos, // Missing account info
            )
            .unwrap_err(),
            AccountResolutionError::IncorrectAccount.into() // Note the error
        );

        // Success
        add_extra_accounts_for_execute_cpi(
            &mut cpi_instruction,
            &mut cpi_account_infos,
            &transfer_hook_program_id,
            source_account_info.clone(),
            mint_account_info.clone(),
            destination_account_info.clone(),
            authority_account_info.clone(),
            amount,
            &additional_account_infos,
        )
        .unwrap();

        let check_metas = [
            AccountMeta::new(source_pubkey, false),
            AccountMeta::new_readonly(mint_pubkey, false),
            AccountMeta::new(destination_pubkey, false),
            AccountMeta::new_readonly(authority_pubkey, true),
            AccountMeta::new_readonly(EXTRA_META_1, true),
            AccountMeta::new_readonly(EXTRA_META_2, true),
            AccountMeta::new(extra_meta_3_pubkey, false),
            AccountMeta::new(extra_meta_4_pubkey, false),
            AccountMeta::new_readonly(validate_state_pubkey, false),
            AccountMeta::new_readonly(transfer_hook_program_id, false),
        ];

        let check_account_infos = vec![
            source_account_info,
            mint_account_info,
            destination_account_info,
            authority_account_info,
            extra_meta_1_account_info,
            extra_meta_2_account_info,
            extra_meta_3_account_info,
            extra_meta_4_account_info,
            validate_state_account_info,
            transfer_hook_program_account_info,
        ];

        assert_eq!(cpi_instruction.accounts, check_metas);
        for (a, b) in std::iter::zip(cpi_account_infos, check_account_infos) {
            assert_eq!(a.key, b.key);
            assert_eq!(a.is_signer, b.is_signer);
            assert_eq!(a.is_writable, b.is_writable);
        }
    }
}
