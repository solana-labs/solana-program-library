//! A convenience module for the `TransferHookManager` -
//! which aids in writing and loading additional account metas
//! for a transfer hook interface program.
use {
    crate::{
        collect_extra_account_metas_signer_seeds, error::TransferHookError,
        get_extra_account_metas_address, get_extra_account_metas_address_and_bump_seed,
        instruction::ExecuteInstruction,
    },
    arrayref::{array_ref, array_refs},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, program::invoke_signed,
        program_error::ProgramError, program_option::COption, pubkey::Pubkey, system_instruction,
    },
    spl_tlv_account_resolution::state::ExtraAccountMetas,
    spl_type_length_value::state::TlvStateBorrowed,
};

/// A manager struct for convenience dealing with setting up and loading the
/// TransferHook extension and applying associated checks.
pub struct TransferHookManager {}

impl TransferHookManager {
    /// Uses the `TransferHookManager` to write the validation data to a validation account.
    ///
    /// This will apply all necessary checks to the passed in values, and will
    /// only write to the account if all checks pass.
    ///
    /// Checks:
    /// * Ensure the mint has a mint authority.
    /// * Ensure the authority is a signer.
    /// * Ensure the authority is the mint authority.
    /// * Ensure the validation account matches the seeds pattern from the interface lib.
    pub fn write(
        program_id: &Pubkey,
        extra_account_metas_info: &AccountInfo<'_>,
        mint_info: &AccountInfo<'_>,
        authority_info: &AccountInfo<'_>,
        extra_account_infos: &[AccountInfo],
    ) -> ProgramResult {
        // check that the mint authority is valid without fully deserializing
        let mint_authority = InlineSplToken::get_mint_authority(&mint_info.try_borrow_data()?)?;
        let mint_authority = mint_authority.ok_or(TransferHookError::MintHasNoMintAuthority)?;

        // Check signers
        if !authority_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if *authority_info.key != mint_authority {
            return Err(TransferHookError::IncorrectMintAuthority.into());
        }

        // Check validation account
        let expected_validation_address =
            get_extra_account_metas_address(mint_info.key, program_id);
        if expected_validation_address != *extra_account_metas_info.key {
            return Err(ProgramError::InvalidSeeds);
        }

        // Create the account
        let (_, bump_seed) =
            get_extra_account_metas_address_and_bump_seed(mint_info.key, program_id);
        let bump_seed = [bump_seed];
        let signer_seeds = collect_extra_account_metas_signer_seeds(mint_info.key, &bump_seed);
        let length = extra_account_infos.len();
        let account_size = ExtraAccountMetas::size_of(length)?;
        invoke_signed(
            &system_instruction::allocate(extra_account_metas_info.key, account_size as u64),
            &[extra_account_metas_info.clone()],
            &[&signer_seeds],
        )?;
        invoke_signed(
            &system_instruction::assign(extra_account_metas_info.key, program_id),
            &[extra_account_metas_info.clone()],
            &[&signer_seeds],
        )?;

        // Write the data
        let mut data = extra_account_metas_info.try_borrow_mut_data()?;
        ExtraAccountMetas::init_with_account_infos::<ExecuteInstruction>(
            &mut data,
            extra_account_infos,
        )?;

        Ok(())
    }

    /// Uses the `TransferHookManager` to assess checks on a provided validation account.
    ///
    /// The extra account metas must be passed in, so they can be checked against
    /// the loaded extra account metas from the validation account.
    ///
    /// Throws a `ProgramError` if they don't match.
    pub fn check(
        program_id: &Pubkey,
        extra_account_metas_info: &AccountInfo<'_>,
        mint_info: &AccountInfo<'_>,
        extra_account_infos: &[AccountInfo],
    ) -> ProgramResult {
        // Check that the correct pda and validation pubkeys are provided
        let expected_validation_address =
            get_extra_account_metas_address(mint_info.key, program_id);
        if expected_validation_address != *extra_account_metas_info.key {
            return Err(ProgramError::InvalidSeeds);
        }

        let data = extra_account_metas_info.try_borrow_data()?;
        let state = TlvStateBorrowed::unpack(&data).unwrap();
        let extra_account_metas =
            ExtraAccountMetas::unpack_with_tlv_state::<ExecuteInstruction>(&state)?;

        // if incorrect number of are provided, error
        let account_metas = extra_account_metas.data();
        if extra_account_infos.len() != account_metas.len() {
            return Err(TransferHookError::IncorrectAccount.into());
        }

        // Let's assume that they're provided in the correct order
        for (i, account_info) in extra_account_infos.iter().enumerate() {
            if &account_metas[i] != account_info {
                return Err(TransferHookError::IncorrectAccount.into());
            }
        }

        Ok(())
    }
}

// Struct required to verify spl-token-2022 mints.
//
// By copying the required functions here, we avoid a circular dependency
// between spl-token-2022 and this crate.
struct InlineSplToken {}

impl InlineSplToken {
    fn unpack_coption_key(src: &[u8; 36]) -> Result<COption<Pubkey>, ProgramError> {
        let (tag, body) = array_refs![src, 4, 32];
        match *tag {
            [0, 0, 0, 0] => Ok(COption::None),
            [1, 0, 0, 0] => Ok(COption::Some(Pubkey::new_from_array(*body))),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    /// Extract the mint authority from the account bytes
    pub fn get_mint_authority(account_data: &[u8]) -> Result<COption<Pubkey>, ProgramError> {
        const MINT_SIZE: usize = 82;
        if account_data.len() < MINT_SIZE {
            Err(ProgramError::InvalidAccountData)
        } else {
            let mint_authority = array_ref![account_data, 0, 36];
            Self::unpack_coption_key(mint_authority)
        }
    }
}
