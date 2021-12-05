//! Initialize a new user for an Orca farm instruction

use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    hash::Hasher,
    instruction::{AccountMeta, Instruction},
    msg,
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
};

pub fn user_init(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Processing AmmInstruction::UserInit");

    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        user_account,
        user_info_account,
        farm_id,
        farm_program_id,
        _system_program,
        ] = accounts
    {
        if !orca::check_stake_program_id(farm_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }

        let farmer_derived = Pubkey::find_program_address(
            &[
                &farm_id.key.to_bytes(),
                &user_account.key.to_bytes(),
                &spl_token::id().to_bytes(),
            ],
            &orca_farm_program,
        )
        .0;
        if &farmer_derived != user_info_account.key {
            msg!("Error: Invalid Farmer address");
            return Err(ProgramError::InvalidSeeds);
        }

        let orca_accounts = vec![
            AccountMeta::new_readonly(*farm_id.key, false),
            AccountMeta::new(*user_info_account.key, false),
            AccountMeta::new_readonly(*user_account.key, true),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let instruction = Instruction {
            program_id: *farm_program_id.key,
            accounts: orca_accounts,
            data: OrcaUserInit {}.to_vec()?,
        };

        invoke(&instruction, accounts)?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    msg!("AmmInstruction::UserInit complete");
    Ok(())
}
