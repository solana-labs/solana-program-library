//! Harvest rewards from an Orca farm instruction

use {
    solana_farm_sdk::{
        instruction::orca::OrcaHarvest,
        program::{account, protocol::orca},
    },
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        msg,
        program::invoke,
        program_error::ProgramError,
    },
};

pub fn harvest(accounts: &[AccountInfo]) -> ProgramResult {
    msg!("Processing AmmInstruction::Harvest");

    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        user_account,
        user_info_account,
        user_reward_token_account,
        farm_program_id,
        base_token_vault,
        reward_token_vault,
        _spl_token_id,
        farm_id,
        farm_authority
        ] = accounts
    {
        if !orca::check_stake_program_id(farm_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }

        let initial_reward_token_user_balance =
            account::get_token_balance(user_reward_token_account)?;

        let orca_accounts = vec![
            AccountMeta::new_readonly(*user_account.key, true),
            AccountMeta::new(*farm_id.key, false),
            AccountMeta::new(*user_info_account.key, false),
            AccountMeta::new_readonly(*base_token_vault.key, false),
            AccountMeta::new(*reward_token_vault.key, false),
            AccountMeta::new(*user_reward_token_account.key, false),
            AccountMeta::new_readonly(*farm_authority.key, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ];

        let instruction = Instruction {
            program_id: *farm_program_id.key,
            accounts: orca_accounts,
            data: OrcaHarvest {}.to_vec()?,
        };
        invoke(&instruction, accounts)?;

        let _ = account::get_balance_increase(
            user_reward_token_account,
            initial_reward_token_user_balance,
        )?;
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    msg!("AmmInstruction::Stake complete");
    Ok(())
}
