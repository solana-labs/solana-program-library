//! Offchain helper for fetching required accounts to build instructions

pub use spl_transfer_hook_interface::offchain::{AccountDataResult, AccountFetchError};
use {
    crate::{
        extension::{transfer_hook, StateWithExtensions},
        state::Mint,
    },
    solana_program::{instruction::Instruction, program_error::ProgramError, pubkey::Pubkey},
    spl_transfer_hook_interface::offchain::add_extra_account_metas_for_execute,
    std::future::Future,
};

/// Offchain helper to create a `TransferChecked` instruction with all
/// additional required account metas for a transfer, including the ones
/// required by the transfer hook.
///
/// To be client-agnostic and to avoid pulling in the full solana-sdk, this
/// simply takes a function that will return its data as `Future<Vec<u8>>` for
/// the given address. Can be called in the following way:
///
/// ```rust,ignore
/// let instruction = create_transfer_checked_instruction_with_extra_metas(
///     &spl_token_2022::id(),
///     &source,
///     &mint,
///     &destination,
///     &authority,
///     &[],
///     amount,
///     decimals,
///     |address| self.client.get_account(&address).map_ok(|opt| opt.map(|acc| acc.data)),
/// )
/// .await?
/// ```
#[allow(clippy::too_many_arguments)]
pub async fn create_transfer_checked_instruction_with_extra_metas<F, Fut>(
    token_program_id: &Pubkey,
    source_pubkey: &Pubkey,
    mint_pubkey: &Pubkey,
    destination_pubkey: &Pubkey,
    authority_pubkey: &Pubkey,
    signer_pubkeys: &[&Pubkey],
    amount: u64,
    decimals: u8,
    fetch_account_data_fn: F,
) -> Result<Instruction, AccountFetchError>
where
    F: Fn(Pubkey) -> Fut,
    Fut: Future<Output = AccountDataResult>,
{
    let mut transfer_instruction = crate::instruction::transfer_checked(
        token_program_id,
        source_pubkey,
        mint_pubkey,
        destination_pubkey,
        authority_pubkey,
        signer_pubkeys,
        amount,
        decimals,
    )?;

    add_extra_account_metas(
        &mut transfer_instruction,
        source_pubkey,
        mint_pubkey,
        destination_pubkey,
        authority_pubkey,
        amount,
        fetch_account_data_fn,
    )
    .await?;

    Ok(transfer_instruction)
}

/// Offchain helper to add required account metas to an instruction, including
/// the ones required by the transfer hook.
///
/// To be client-agnostic and to avoid pulling in the full solana-sdk, this
/// simply takes a function that will return its data as `Future<Vec<u8>>` for
/// the given address. Can be called in the following way:
///
/// ```rust,ignore
/// let mut transfer_instruction = spl_token_2022::instruction::transfer_checked(
///     &spl_token_2022::id(),
///     source_pubkey,
///     mint_pubkey,
///     destination_pubkey,
///     authority_pubkey,
///     signer_pubkeys,
///     amount,
///     decimals,
/// )?;
/// add_extra_account_metas(
///     &mut transfer_instruction,
///     source_pubkey,
///     mint_pubkey,
///     destination_pubkey,
///     authority_pubkey,
///     amount,
///     fetch_account_data_fn,
/// ).await?;
/// ```
pub async fn add_extra_account_metas<F, Fut>(
    instruction: &mut Instruction,
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
    let mint_data = fetch_account_data_fn(*mint_pubkey)
        .await?
        .ok_or(ProgramError::InvalidAccountData)?;
    let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;

    if let Some(program_id) = transfer_hook::get_program_id(&mint) {
        add_extra_account_metas_for_execute(
            instruction,
            &program_id,
            source_pubkey,
            mint_pubkey,
            destination_pubkey,
            authority_pubkey,
            amount,
            fetch_account_data_fn,
        )
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::extension::{transfer_hook::TransferHook, ExtensionType, StateWithExtensionsMut},
        solana_program::{instruction::AccountMeta, program_option::COption},
        solana_program_test::tokio,
        spl_pod::optional_keys::OptionalNonZeroPubkey,
        spl_tlv_account_resolution::{
            account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList,
        },
        spl_transfer_hook_interface::{
            get_extra_account_metas_address, instruction::ExecuteInstruction,
        },
    };

    const DECIMALS: u8 = 0;
    const MINT_PUBKEY: Pubkey = Pubkey::new_from_array([1u8; 32]);
    const TRANSFER_HOOK_PROGRAM_ID: Pubkey = Pubkey::new_from_array([2u8; 32]);
    const EXTRA_META_1: Pubkey = Pubkey::new_from_array([3u8; 32]);
    const EXTRA_META_2: Pubkey = Pubkey::new_from_array([4u8; 32]);

    // Mock to return the mint data or the validation state account data
    async fn mock_fetch_account_data_fn(address: Pubkey) -> AccountDataResult {
        if address == MINT_PUBKEY {
            let mint_len =
                ExtensionType::try_calculate_account_len::<Mint>(&[ExtensionType::TransferHook])
                    .unwrap();
            let mut data = vec![0u8; mint_len];
            let mut mint = StateWithExtensionsMut::<Mint>::unpack_uninitialized(&mut data).unwrap();

            let extension = mint.init_extension::<TransferHook>(true).unwrap();
            extension.program_id =
                OptionalNonZeroPubkey::try_from(Some(TRANSFER_HOOK_PROGRAM_ID)).unwrap();

            mint.base.mint_authority = COption::Some(Pubkey::new_unique());
            mint.base.decimals = DECIMALS;
            mint.base.is_initialized = true;
            mint.base.freeze_authority = COption::None;
            mint.pack_base();
            mint.init_account_type().unwrap();

            Ok(Some(data))
        } else if address
            == get_extra_account_metas_address(&MINT_PUBKEY, &TRANSFER_HOOK_PROGRAM_ID)
        {
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
        } else {
            Ok(None)
        }
    }

    #[tokio::test]
    async fn test_create_transfer_checked_instruction_with_extra_metas() {
        let source = Pubkey::new_unique();
        let destination = Pubkey::new_unique();
        let authority = Pubkey::new_unique();
        let amount = 100u64;

        let validate_state_pubkey =
            get_extra_account_metas_address(&MINT_PUBKEY, &TRANSFER_HOOK_PROGRAM_ID);
        let extra_meta_3_pubkey = Pubkey::find_program_address(
            &[
                source.as_ref(),
                destination.as_ref(),
                validate_state_pubkey.as_ref(),
            ],
            &TRANSFER_HOOK_PROGRAM_ID,
        )
        .0;
        let extra_meta_4_pubkey = Pubkey::find_program_address(
            &[
                amount.to_le_bytes().as_ref(),
                destination.as_ref(),
                EXTRA_META_1.as_ref(),
                extra_meta_3_pubkey.as_ref(),
            ],
            &TRANSFER_HOOK_PROGRAM_ID,
        )
        .0;

        let instruction = create_transfer_checked_instruction_with_extra_metas(
            &crate::id(),
            &source,
            &MINT_PUBKEY,
            &destination,
            &authority,
            &[],
            amount,
            DECIMALS,
            mock_fetch_account_data_fn,
        )
        .await
        .unwrap();

        let check_metas = [
            AccountMeta::new(source, false),
            AccountMeta::new_readonly(MINT_PUBKEY, false),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new_readonly(EXTRA_META_1, true),
            AccountMeta::new_readonly(EXTRA_META_2, true),
            AccountMeta::new(extra_meta_3_pubkey, false),
            AccountMeta::new(extra_meta_4_pubkey, false),
            AccountMeta::new_readonly(TRANSFER_HOOK_PROGRAM_ID, false),
            AccountMeta::new_readonly(validate_state_pubkey, false),
        ];

        assert_eq!(instruction.accounts, check_metas);

        // With additional signers
        let signer_1 = Pubkey::new_unique();
        let signer_2 = Pubkey::new_unique();
        let signer_3 = Pubkey::new_unique();

        let instruction = create_transfer_checked_instruction_with_extra_metas(
            &crate::id(),
            &source,
            &MINT_PUBKEY,
            &destination,
            &authority,
            &[&signer_1, &signer_2, &signer_3],
            amount,
            DECIMALS,
            mock_fetch_account_data_fn,
        )
        .await
        .unwrap();

        let check_metas = [
            AccountMeta::new(source, false),
            AccountMeta::new_readonly(MINT_PUBKEY, false),
            AccountMeta::new(destination, false),
            AccountMeta::new_readonly(authority, false), // False because of additional signers
            AccountMeta::new_readonly(signer_1, true),
            AccountMeta::new_readonly(signer_2, true),
            AccountMeta::new_readonly(signer_3, true),
            AccountMeta::new_readonly(EXTRA_META_1, true),
            AccountMeta::new_readonly(EXTRA_META_2, true),
            AccountMeta::new(extra_meta_3_pubkey, false),
            AccountMeta::new(extra_meta_4_pubkey, false),
            AccountMeta::new_readonly(TRANSFER_HOOK_PROGRAM_ID, false),
            AccountMeta::new_readonly(validate_state_pubkey, false),
        ];

        assert_eq!(instruction.accounts, check_metas);
    }
}
