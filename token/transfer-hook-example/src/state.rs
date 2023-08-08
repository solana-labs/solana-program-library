//! State helpers for working with the example program

use {
    solana_program::{
        account_info::AccountInfo, instruction::AccountMeta, program_error::ProgramError,
        pubkey::Pubkey, sysvar,
    },
    spl_tlv_account_resolution::{pod::PodAccountMeta, seeds::Seed, state::ExtraAccountMetas},
    spl_transfer_hook_interface::{error::TransferHookError, instruction::ExecuteInstruction},
};

/// Generate example data to be used directly in an account for testing
pub fn example_data(account_metas: &[AccountMeta]) -> Result<Vec<u8>, ProgramError> {
    let account_size = ExtraAccountMetas::size_of(account_metas.len())?;
    let mut data = vec![0; account_size];
    ExtraAccountMetas::init_with_account_metas::<ExecuteInstruction>(&mut data, account_metas)?;
    Ok(data)
}

/// Create the validation data (the extra required accounts configs)
///
/// Note: since the PDA is not known at the time of intialization
/// of the extra required accounts config, we only need to make sure
/// the account metas were provided in the instruction.
pub fn create_validation_data(
    remaining_account_infos: &[AccountInfo],
    mint_authority: &Pubkey,
) -> Result<[PodAccountMeta; 4], ProgramError> {
    if !(remaining_account_infos.len() == 3 // 3 metas and one PDA
        && remaining_account_infos[0].key == &sysvar::instructions::id()
        && remaining_account_infos[1].key == mint_authority)
    {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    Ok([
        PodAccountMeta::from(&remaining_account_infos[0]),
        PodAccountMeta::from(&remaining_account_infos[1]),
        // Third required account: PDA
        PodAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: b"transfer-hook-example".to_vec(),
                },
                Seed::AccountKey { index: 0 }, // Source account's key
            ],
            false,
            false,
        )?,
        PodAccountMeta::from(&remaining_account_infos[2]),
    ])
}

/// As an example, just checks that all the required additional accounts
/// were provided
pub fn validate_extra_provided_accounts(
    all_account_infos: &[AccountInfo],
    required_extra_account_metas: &[PodAccountMeta],
    program_id: &Pubkey,
) -> Result<(), ProgramError> {
    for pod_meta in required_extra_account_metas {
        let meta = ExtraAccountMetas::resolve_account_meta::<AccountInfo>(
            pod_meta,
            all_account_infos,
            &[], // No instruction data used in this example
            program_id,
        )?;
        if !all_account_infos.iter().any(|info| {
            if info.key == &meta.pubkey {
                info.is_signer == meta.is_signer && info.is_writable == meta.is_writable
            } else {
                false
            }
        }) {
            return Err(TransferHookError::IncorrectAccount.into());
        }
    }
    Ok(())
}
