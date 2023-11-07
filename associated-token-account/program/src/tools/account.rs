//! Account utility functions

use {
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        program::{get_return_data, invoke, invoke_signed},
        program_error::ProgramError,
        pubkey::Pubkey,
        rent::Rent,
        system_instruction,
    },
    spl_token_2022::extension::ExtensionType,
    std::convert::TryInto,
};

/// Creates associated token account using Program Derived Address for the given
/// seeds
pub fn create_pda_account<'a>(
    payer: &AccountInfo<'a>,
    rent: &Rent,
    space: usize,
    owner: &Pubkey,
    system_program: &AccountInfo<'a>,
    new_pda_account: &AccountInfo<'a>,
    new_pda_signer_seeds: &[&[u8]],
) -> ProgramResult {
    if new_pda_account.lamports() > 0 {
        let required_lamports = rent
            .minimum_balance(space)
            .max(1)
            .saturating_sub(new_pda_account.lamports());

        if required_lamports > 0 {
            invoke(
                &system_instruction::transfer(payer.key, new_pda_account.key, required_lamports),
                &[
                    payer.clone(),
                    new_pda_account.clone(),
                    system_program.clone(),
                ],
            )?;
        }

        invoke_signed(
            &system_instruction::allocate(new_pda_account.key, space as u64),
            &[new_pda_account.clone(), system_program.clone()],
            &[new_pda_signer_seeds],
        )?;

        invoke_signed(
            &system_instruction::assign(new_pda_account.key, owner),
            &[new_pda_account.clone(), system_program.clone()],
            &[new_pda_signer_seeds],
        )
    } else {
        invoke_signed(
            &system_instruction::create_account(
                payer.key,
                new_pda_account.key,
                rent.minimum_balance(space).max(1),
                space as u64,
                owner,
            ),
            &[
                payer.clone(),
                new_pda_account.clone(),
                system_program.clone(),
            ],
            &[new_pda_signer_seeds],
        )
    }
}

/// Determines the required initial data length for a new token account based on
/// the extensions initialized on the Mint
pub fn get_account_len<'a>(
    mint: &AccountInfo<'a>,
    spl_token_program: &AccountInfo<'a>,
    extension_types: &[ExtensionType],
) -> Result<usize, ProgramError> {
    invoke(
        &spl_token_2022::instruction::get_account_data_size(
            spl_token_program.key,
            mint.key,
            extension_types,
        )?,
        &[mint.clone(), spl_token_program.clone()],
    )?;
    get_return_data()
        .ok_or(ProgramError::InvalidInstructionData)
        .and_then(|(key, data)| {
            if key != *spl_token_program.key {
                return Err(ProgramError::IncorrectProgramId);
            }
            data.try_into()
                .map(usize::from_le_bytes)
                .map_err(|_| ProgramError::InvalidInstructionData)
        })
}
