//! Offchain helper for fetching required accounts to build instructions

pub use spl_tlv_account_resolution::state::{AccountDataResult, AccountFetchError};
use {
    crate::{
        error::TransferHookError,
        get_extra_account_metas_address,
        instruction::{execute, ExecuteInstruction},
    },
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_tlv_account_resolution::state::ExtraAccountMetaList,
    std::future::Future,
};

/// Offchain helper to get all additional required account metas for an execute
/// instruction, based on a validation state account.
///
/// The instruction being provided to this function must contain at least the
/// same account keys as the ones being provided, in order. Specifically:
/// 1. source
/// 2. mint
/// 3. destination
/// 4. authority
///
/// The `program_id` should be the program ID of the program that the
/// created `ExecuteInstruction` is for.
///
/// To be client-agnostic and to avoid pulling in the full solana-sdk, this
/// simply takes a function that will return its data as `Future<Vec<u8>>` for
/// the given address. Can be called in the following way:
///
/// ```rust,ignore
/// add_extra_account_metas_for_execute(
///     &mut instruction,
///     &program_id,
///     &source,
///     &mint,
///     &destination,
///     &authority,
///     amount,
///     |address| self.client.get_account(&address).map_ok(|opt| opt.map(|acc| acc.data)),
/// )
/// .await?;
/// ```
#[allow(clippy::too_many_arguments)]
pub async fn add_extra_account_metas_for_execute<F, Fut>(
    instruction: &mut Instruction,
    program_id: &Pubkey,
    source_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    amount: u64,
    fetch_account_data_fn: F,
) -> Result<(), AccountFetchError>
where
    F: Fn(Pubkey) -> Fut,
    Fut: Future<Output = AccountDataResult>,
{
    let validate_state_pubkey = get_extra_account_metas_address(mint_pubkey, program_id);
    let validate_state_data = fetch_account_data_fn(validate_state_pubkey)
        .await?
        .ok_or(ProgramError::InvalidAccountData)?;

    // Check to make sure the provided keys are in the instruction
    if [
        source_pubkey,
        mint_pubkey,
        destination_pubkey,
        authority_pubkey,
    ]
    .iter()
    .any(|&key| !instruction.accounts.iter().any(|meta| meta.pubkey == *key))
    {
        Err(TransferHookError::IncorrectAccount)?;
    }

    let mut execute_instruction = execute(
        program_id,
        source_pubkey,
        mint_pubkey,
        destination_pubkey,
        authority_pubkey,
        amount,
    );
    execute_instruction
        .accounts
        .push(AccountMeta::new_readonly(validate_state_pubkey, false));

    ExtraAccountMetaList::add_to_instruction::<ExecuteInstruction, _, _>(
        &mut execute_instruction,
        fetch_account_data_fn,
        &validate_state_data,
    )
    .await?;

    // Add only the extra accounts resolved from the validation state
    instruction
        .accounts
        .extend_from_slice(&execute_instruction.accounts[5..]);

    // Add the program id and validation state account
    instruction
        .accounts
        .push(AccountMeta::new_readonly(*program_id, false));
    instruction
        .accounts
        .push(AccountMeta::new_readonly(validate_state_pubkey, false));

    Ok(())
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        spl_tlv_account_resolution::{account::ExtraAccountMeta, seeds::Seed},
        tokio,
    };

    const PROGRAM_ID: Pubkey = Pubkey::new_from_array([1u8; 32]);
    const EXTRA_META_1: Pubkey = Pubkey::new_from_array([2u8; 32]);
    const EXTRA_META_2: Pubkey = Pubkey::new_from_array([3u8; 32]);

    // Mock to return the validation state account data
    async fn mock_fetch_account_data_fn(_address: Pubkey) -> AccountDataResult {
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
        ExtraAccountMetaList::init::<ExecuteInstruction>(&mut data, &extra_metas)?;
        Ok(Some(data))
    }

    #[tokio::test]
    async fn test_add_extra_account_metas_for_execute() {
        let source = Pubkey::new_unique();
        let mint = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let amount = 100u64;

        let validate_state_pubkey = get_extra_account_metas_address(&mint, &PROGRAM_ID);
        let extra_meta_3_pubkey = Pubkey::find_program_address(
            &[
                source.as_ref(),
                destination.as_ref(),
                validate_state_pubkey.as_ref(),
            ],
            &PROGRAM_ID,
        )
        .0;
        let extra_meta_4_pubkey = Pubkey::find_program_address(
            &[
                amount.to_le_bytes().as_ref(),
                destination.as_ref(),
                EXTRA_META_1.as_ref(),
                extra_meta_3_pubkey.as_ref(),
            ],
            &PROGRAM_ID,
        )
        .0;

        // Fail missing key
        let mut instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &[],
            vec![
                // source missing
                AccountMeta::new_readonly(mint, false),
                AccountMeta::new(destination, false),
                AccountMeta::new_readonly(authority, true),
            ],
        );
        assert_eq!(
            add_extra_account_metas_for_execute(
                &mut instruction,
                &PROGRAM_ID,
                &source,
                &mint,
                &destination,
                &authority,
                amount,
                mock_fetch_account_data_fn,
            )
            .await
            .unwrap_err()
            .downcast::<TransferHookError>()
            .unwrap(),
            Box::new(TransferHookError::IncorrectAccount)
        );

        // Success
        let mut instruction = Instruction::new_with_bytes(
            PROGRAM_ID,
            &[],
            vec![
                AccountMeta::new(source, false),
                AccountMeta::new_readonly(mint, false),
                AccountMeta::new(destination, false),
                AccountMeta::new_readonly(authority, true),
            ],
        );
        add_extra_account_metas_for_execute(
            &mut instruction,
            &PROGRAM_ID,
            &source,
            &mint,
            &destination,
            &authority,
            amount,
            mock_fetch_account_data_fn,
        )
        .await
        .unwrap();

        let check_metas = [
            AccountMeta::new(source, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(EXTRA_META_1, true),
            AccountMeta::new_readonly(EXTRA_META_2, true),
            AccountMeta::new(extra_meta_3_pubkey, false),
            AccountMeta::new(extra_meta_4_pubkey, false),
            AccountMeta::new_readonly(PROGRAM_ID, false),
            AccountMeta::new_readonly(validate_state_pubkey, false),
        ];

        assert_eq!(instruction.accounts, check_metas);
    }
}
