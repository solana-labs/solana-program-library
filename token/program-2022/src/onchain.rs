//! On-chain program invoke helper to perform on-chain `transfer_checked` with
//! correct accounts

use {
    crate::{
        error::TokenError,
        extension::{transfer_hook, StateWithExtensions},
        instruction,
        state::Mint,
    },
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        msg,
        program::invoke_signed,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_transfer_hook_interface::{
        error::TransferHookError, get_extra_account_metas_address,
        onchain::add_cpi_accounts_for_execute,
    },
};

/// Onchain helper to get all additional required account metas for a checked
/// transfer
///
/// Note that this onchain helper will build a new `Execute` instruction,
/// resolve the extra account metas, and then add them to the transfer
/// instruction. This is because the extra account metas are configured
/// specifically for the `Execute` instruction, which requires five accounts
/// (source, mint, destination, authority, and validation state), wheras the
/// transfer instruction only requires four (source, mint, destination, and
/// authority) in addition to `n` number of multisig authorities.
pub fn resolve_extra_transfer_account_metas_for_cpi<'a>(
    cpi_instruction: &mut Instruction,
    cpi_account_infos: &mut Vec<AccountInfo<'a>>,
    mint_info: &AccountInfo<'a>,
    additional_accounts: &[AccountInfo<'a>],
    amount: u64,
) -> Result<(), ProgramError> {
    let mint_data = mint_info.try_borrow_data()?;
    let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;
    if let Some(program_id) = transfer_hook::get_program_id(&mint) {
        // Convert the transfer instruction into an `Execute` instruction,
        // then resolve the extra account metas as configured in the validation
        // account data, then finally add the extra account metas to the original
        // transfer instruction.
        if cpi_instruction.accounts.len() < 4 {
            msg!("Not a valid transfer instruction");
            Err(TokenError::InvalidInstruction)?;
        }

        let validation_pubkey = get_extra_account_metas_address(mint_info.key, &program_id);
        let validation_info = additional_accounts
            .iter()
            .find(|&x| *x.key == validation_pubkey)
            .ok_or(TransferHookError::IncorrectAccount)?;

        let mut execute_ix = spl_transfer_hook_interface::instruction::execute(
            &program_id,
            &cpi_instruction.accounts[0].pubkey,
            &cpi_instruction.accounts[1].pubkey,
            &cpi_instruction.accounts[2].pubkey,
            &cpi_instruction.accounts[3].pubkey,
            &validation_pubkey,
            amount,
        );

        cpi_account_infos.push(validation_info.clone());

        add_cpi_accounts_for_execute(
            &mut execute_ix,
            cpi_account_infos,
            mint_info.key,
            &program_id,
            additional_accounts,
        )?;

        cpi_instruction
            .accounts
            .extend_from_slice(&execute_ix.accounts[5..]);
    }
    Ok(())
}

/// Helper to CPI into token-2022 on-chain, looking through the additional
/// account infos to create the proper instruction with the proper account
/// infos.
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
        source_info,
        mint_info.clone(),
        destination_info,
        authority_info,
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

    resolve_extra_transfer_account_metas_for_cpi(
        &mut cpi_instruction,
        &mut cpi_account_infos,
        &mint_info,
        additional_accounts,
        amount,
    )?;

    invoke_signed(&cpi_instruction, &cpi_account_infos, seeds)
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        solana_program::system_program,
        solana_program_test::tokio,
        spl_tlv_account_resolution::state::{AccountDataResult, ExtraAccountMetaList},
        spl_transfer_hook_interface::instruction::ExecuteInstruction,
    };

    const TRANSFER_HOOK_PROGRAM_ID: Pubkey = Pubkey::new_from_array([1; 32]);

    const MINT_PUBKEY: Pubkey = Pubkey::new_from_array([2; 32]);

    const MOCK_MINT_STATE: [u8; 234] = [
        0, 0, 0, 0, // COption (4): None = 0
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, // Mint authority (32)
        0, 0, 0, 0, 0, 0, 0, 0, // Supply (8)
        0, // Decimals (1)
        1, // Is initialized (1)
        0, 0, 0, 0, // COption (4): None = 0
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, // Freeze authority (32)
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // Padding (83)
        1, // Account type (1): Mint = 1
        14, 0, // Extension type (2): Transfer hook = 14
        64, 0, // Extension length (2): 64
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, // Authority (32)
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1, // Transfer hook program ID (32)
    ];

    const MOCK_EXTRA_METAS_STATE: [u8; 226] = [
        105, 37, 101, 197, 75, 251, 102, 26, // Discriminator for `ExecuteInstruction` (8)
        214, 0, 0, 0, // Length of pod slice (4): 214
        6, 0, 0, 0, // Count of account metas (4): 6
        1, // First account meta discriminator (1): PDA = 1
        3, 0, // First seed: Account key at index 0 (2)
        3, 1, // Second seed: Account key at index 1 (2)
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, // No more seeds (28)
        0, // First account meta is signer (1): false = 0
        0, // First account meta is writable (1): false = 0
        1, // Second account meta discriminator (1): PDA = 1
        3, 4, // First seed: Account key at index 4 (2)
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, // No more seeds (30)
        0, // Second account meta is signer (1): false = 0
        0, // Second account meta is writable (1): false = 0
        1, // Third account meta discriminator (1): PDA = 1
        1, 6, 112, 114, 101, 102, 105, 120, // First seed: Literal "prefix" (8)
        2, 8, 8, // Second seed: Instruction data 8..16 (3)
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // No more seeds (21)
        0, // Third account meta is signer (1): false = 0
        0, // Third account meta is writable (1): false = 0
        0, // Fourth account meta discriminator (1): Pubkey = 0
        7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
        7, 7,   // Pubkey (32)
        0,   // Fourth account meta is signer (1): false = 0
        0,   // Fourth account meta is writable (1): false = 0
        136, // Fifth account meta discriminator (1): External PDA = 128 + index 8 = 136
        1, 6, 112, 114, 101, 102, 105, 120, // First seed: Literal "prefix" (8)
        2, 8, 8, // Second seed: Instruction data 8..16 (3)
        3, 6, // Third seed: Account key at index 6 (2)
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,   // No more seeds (19)
        0,   // Fifth account meta is signer (1): false = 0
        0,   // Fifth account meta is writable (1): false = 0
        136, // Sixth account meta discriminator (1): External PDA = 128 + index 8 = 136
        1, 14, 97, 110, 111, 116, 104, 101, 114, 95, 112, 114, 101, 102, 105,
        120, // First seed: Literal "another_prefix" (16)
        2, 8, 8, // Second seed: Instruction data 8..16 (3)
        3, 6, // Third seed: Account key at index 6 (2)
        3, 9, // Fourth seed: Account key at index 9 (2)
        0, 0, 0, 0, 0, 0, 0, 0, 0, // No more seeds (9)
        0, // Sixth account meta is signer (1): false = 0
        0, // Sixth account meta is writable (1): false = 0
    ];

    async fn mock_fetch_account_data_fn(address: Pubkey) -> AccountDataResult {
        if address == MINT_PUBKEY {
            Ok(Some(MOCK_MINT_STATE.to_vec()))
        } else if address
            == get_extra_account_metas_address(&MINT_PUBKEY, &TRANSFER_HOOK_PROGRAM_ID)
        {
            Ok(Some(MOCK_EXTRA_METAS_STATE.to_vec()))
        } else {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_resolve_extra_transfer_account_metas_for_cpi() {
        let spl_token_2022_program_id = crate::id();
        let transfer_hook_program_id = TRANSFER_HOOK_PROGRAM_ID;
        let amount = 2u64;

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

        let mint_pubkey = MINT_PUBKEY;
        let mut mint_data = MOCK_MINT_STATE.to_vec();
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

        let extra_meta_1_pubkey = Pubkey::find_program_address(
            &[
                &source_pubkey.to_bytes(), // Account key at index 0
                &mint_pubkey.to_bytes(),   // Account key at index 1
            ],
            &transfer_hook_program_id,
        )
        .0;
        let mut extra_meta_1_data = vec![]; // Mock
        let mut extra_meta_1_lamports = 0; // Mock
        let extra_meta_1_account_info = AccountInfo::new(
            &extra_meta_1_pubkey,
            false,
            true,
            &mut extra_meta_1_lamports,
            &mut extra_meta_1_data,
            &transfer_hook_program_id,
            false,
            0,
        );

        let extra_meta_2_pubkey = Pubkey::find_program_address(
            &[
                &validate_state_pubkey.to_bytes(), // Account key at index 4
            ],
            &transfer_hook_program_id,
        )
        .0;
        let mut extra_meta_2_data = vec![]; // Mock
        let mut extra_meta_2_lamports = 0; // Mock
        let extra_meta_2_account_info = AccountInfo::new(
            &extra_meta_2_pubkey,
            false,
            true,
            &mut extra_meta_2_lamports,
            &mut extra_meta_2_data,
            &transfer_hook_program_id,
            false,
            0,
        );

        let extra_meta_3_pubkey = Pubkey::find_program_address(
            &[
                b"prefix",
                amount.to_le_bytes().as_ref(), // Instruction data 8..16
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

        let extra_meta_4_pubkey = Pubkey::new_from_array([7; 32]); // Some arbitrary program ID
        let mut extra_meta_4_data = vec![]; // Mock
        let mut extra_meta_4_lamports = 0; // Mock
        let extra_meta_4_account_info = AccountInfo::new(
            &extra_meta_4_pubkey,
            false,
            true,
            &mut extra_meta_4_lamports,
            &mut extra_meta_4_data,
            &transfer_hook_program_id,
            true, // Executable program
            0,
        );

        let extra_meta_5_pubkey = Pubkey::find_program_address(
            &[
                b"prefix",
                amount.to_le_bytes().as_ref(), // Instruction data 8..16
                extra_meta_2_pubkey.as_ref(),
            ],
            &extra_meta_4_pubkey, // PDA off of the arbitrary program ID
        )
        .0;
        let mut extra_meta_5_data = vec![]; // Mock
        let mut extra_meta_5_lamports = 0; // Mock
        let extra_meta_5_account_info = AccountInfo::new(
            &extra_meta_5_pubkey,
            false,
            true,
            &mut extra_meta_5_lamports,
            &mut extra_meta_5_data,
            &extra_meta_4_pubkey,
            false,
            0,
        );

        let extra_meta_6_pubkey = Pubkey::find_program_address(
            &[
                b"another_prefix",
                amount.to_le_bytes().as_ref(), // Instruction data 8..16
                extra_meta_2_pubkey.as_ref(),
                extra_meta_5_pubkey.as_ref(),
            ],
            &extra_meta_4_pubkey, // PDA off of the arbitrary program ID
        )
        .0;
        let mut extra_meta_6_data = vec![]; // Mock
        let mut extra_meta_6_lamports = 0; // Mock
        let extra_meta_6_account_info = AccountInfo::new(
            &extra_meta_6_pubkey,
            false,
            true,
            &mut extra_meta_6_lamports,
            &mut extra_meta_6_data,
            &extra_meta_4_pubkey,
            false,
            0,
        );

        let mut validate_state_data = MOCK_EXTRA_METAS_STATE.to_vec();
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
        let transfer_hook_program_info = AccountInfo::new(
            &transfer_hook_program_id,
            false,
            true,
            &mut transfer_hook_program_lamports,
            &mut transfer_hook_program_data,
            &system_program::ID,
            true, // Executable program
            0,
        );

        // First use the resolve function to add the extra account metas to the
        // transfer instruction from onchain
        let mut onchain_transfer_cpi_instruction = crate::instruction::transfer_checked(
            &spl_token_2022_program_id,
            &source_pubkey,
            &mint_pubkey,
            &destination_pubkey,
            &authority_pubkey,
            &[],
            amount,
            9,
        )
        .unwrap();
        let mut onchain_transfer_cpi_account_infos = vec![
            source_account_info.clone(),
            mint_account_info.clone(),
            destination_account_info.clone(),
            authority_account_info.clone(),
        ];
        let onchain_transfer_additional_account_infos = vec![
            extra_meta_1_account_info.clone(),
            extra_meta_2_account_info.clone(),
            extra_meta_3_account_info.clone(),
            extra_meta_4_account_info.clone(),
            extra_meta_5_account_info.clone(),
            extra_meta_6_account_info.clone(),
            validate_state_account_info.clone(),
            transfer_hook_program_info.clone(),
        ];

        resolve_extra_transfer_account_metas_for_cpi(
            &mut onchain_transfer_cpi_instruction,
            &mut onchain_transfer_cpi_account_infos,
            &mint_account_info,
            &onchain_transfer_additional_account_infos,
            amount,
        )
        .unwrap();

        // Then use the offchain function to add the extra account metas to the
        // _execute_ instruction from offchain
        let mut offchain_execute_instruction = spl_transfer_hook_interface::instruction::execute(
            &transfer_hook_program_id,
            &source_pubkey,
            &mint_pubkey,
            &destination_pubkey,
            &authority_pubkey,
            &validate_state_pubkey,
            amount,
        );

        ExtraAccountMetaList::add_to_instruction::<ExecuteInstruction, _, _>(
            &mut offchain_execute_instruction,
            mock_fetch_account_data_fn,
            &MOCK_EXTRA_METAS_STATE,
        )
        .await
        .unwrap();

        // Finally, use the onchain function to add the extra account metas to
        // the _execute_ CPI instruction from onchain
        let mut onchain_execute_cpi_instruction = spl_transfer_hook_interface::instruction::execute(
            &transfer_hook_program_id,
            &source_pubkey,
            &mint_pubkey,
            &destination_pubkey,
            &authority_pubkey,
            &validate_state_pubkey,
            amount,
        );
        let mut onchain_execute_cpi_account_infos = vec![
            source_account_info.clone(),
            mint_account_info.clone(),
            destination_account_info.clone(),
            authority_account_info.clone(),
            validate_state_account_info.clone(),
        ];
        let all_account_infos = &[
            source_account_info.clone(),
            mint_account_info.clone(),
            destination_account_info.clone(),
            authority_account_info.clone(),
            validate_state_account_info.clone(),
            extra_meta_1_account_info.clone(),
            extra_meta_2_account_info.clone(),
            extra_meta_3_account_info.clone(),
            extra_meta_4_account_info.clone(),
            extra_meta_5_account_info.clone(),
            extra_meta_6_account_info.clone(),
        ];

        ExtraAccountMetaList::add_to_cpi_instruction::<ExecuteInstruction>(
            &mut onchain_execute_cpi_instruction,
            &mut onchain_execute_cpi_account_infos,
            &MOCK_EXTRA_METAS_STATE,
            all_account_infos,
        )
        .unwrap();

        // The two `Execute` instructions should have the same accounts
        assert_eq!(
            offchain_execute_instruction.accounts,
            onchain_execute_cpi_instruction.accounts,
        );

        // Still, the transfer instruction is going to be missing the
        // the validation account at index 4
        assert_ne!(
            onchain_transfer_cpi_instruction.accounts,
            offchain_execute_instruction.accounts,
        );
        assert_ne!(
            onchain_transfer_cpi_instruction.accounts[4].pubkey,
            validate_state_pubkey,
        );

        // Even though both execute instructions have the validation account
        // at index 4
        assert_eq!(
            offchain_execute_instruction.accounts[4].pubkey,
            validate_state_pubkey,
        );
        assert_eq!(
            onchain_execute_cpi_instruction.accounts[4].pubkey,
            validate_state_pubkey,
        );

        // The most important thing is verifying all PDAs are correct across
        // all lists
        // PDA 1
        assert_eq!(
            onchain_transfer_cpi_instruction.accounts[4].pubkey,
            extra_meta_1_pubkey,
        );
        assert_eq!(
            offchain_execute_instruction.accounts[5].pubkey,
            extra_meta_1_pubkey,
        );
        assert_eq!(
            onchain_execute_cpi_instruction.accounts[5].pubkey,
            extra_meta_1_pubkey,
        );
        // PDA 2
        assert_eq!(
            onchain_transfer_cpi_instruction.accounts[5].pubkey,
            extra_meta_2_pubkey,
        );
        assert_eq!(
            offchain_execute_instruction.accounts[6].pubkey,
            extra_meta_2_pubkey,
        );
        assert_eq!(
            onchain_execute_cpi_instruction.accounts[6].pubkey,
            extra_meta_2_pubkey,
        );
        // PDA 3
        assert_eq!(
            onchain_transfer_cpi_instruction.accounts[6].pubkey,
            extra_meta_3_pubkey,
        );
        assert_eq!(
            offchain_execute_instruction.accounts[7].pubkey,
            extra_meta_3_pubkey,
        );
        assert_eq!(
            onchain_execute_cpi_instruction.accounts[7].pubkey,
            extra_meta_3_pubkey,
        );
        // PDA 4
        assert_eq!(
            onchain_transfer_cpi_instruction.accounts[7].pubkey,
            extra_meta_4_pubkey,
        );
        assert_eq!(
            offchain_execute_instruction.accounts[8].pubkey,
            extra_meta_4_pubkey,
        );
        assert_eq!(
            onchain_execute_cpi_instruction.accounts[8].pubkey,
            extra_meta_4_pubkey,
        );
        // PDA 5
        assert_eq!(
            onchain_transfer_cpi_instruction.accounts[8].pubkey,
            extra_meta_5_pubkey,
        );
        assert_eq!(
            offchain_execute_instruction.accounts[9].pubkey,
            extra_meta_5_pubkey,
        );
        assert_eq!(
            onchain_execute_cpi_instruction.accounts[9].pubkey,
            extra_meta_5_pubkey,
        );
        // PDA 6
        assert_eq!(
            onchain_transfer_cpi_instruction.accounts[9].pubkey,
            extra_meta_6_pubkey,
        );
        assert_eq!(
            offchain_execute_instruction.accounts[10].pubkey,
            extra_meta_6_pubkey,
        );
        assert_eq!(
            onchain_execute_cpi_instruction.accounts[10].pubkey,
            extra_meta_6_pubkey,
        );
    }
}
