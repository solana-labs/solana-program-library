//! Stake LP tokens to a Raydium farm instruction

use {
    solana_farm_sdk::{
        id::zero,
        instruction::raydium::RaydiumStake,
        program::{account, protocol::raydium},
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

pub fn stake(accounts: &[AccountInfo], amount: u64, harvest: bool) -> ProgramResult {
    msg!("Processing AmmInstruction::Stake");
    msg!("amount {} ", amount);

    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        user_account,
        user_info_account,
        user_lp_token_account,
        user_reward_token_a_account,
        user_reward_token_b_account,
        farm_program_id,
        farm_lp_token_account,
        farm_reward_token_a_account,
        farm_reward_token_b_account,
        clock_id,
        spl_token_id,
        farm_id,
        farm_authority
        ] = accounts
    {
        if !raydium::check_stake_program_id(farm_program_id.key) {
            return Err(ProgramError::IncorrectProgramId);
        }
        let dual_rewards = *farm_reward_token_b_account.key != zero::id();
        let initial_token_a_user_balance = account::get_token_balance(user_reward_token_a_account)?;
        let initial_token_b_user_balance = if dual_rewards {
            account::get_token_balance(user_reward_token_b_account)?
        } else {
            0
        };
        let initial_lp_token_user_balance = account::get_token_balance(user_lp_token_account)?;

        let mut raydium_accounts = Vec::with_capacity(12);
        raydium_accounts.push(AccountMeta::new(*farm_id.key, false));
        raydium_accounts.push(AccountMeta::new_readonly(*farm_authority.key, false));
        raydium_accounts.push(AccountMeta::new(*user_info_account.key, false));
        raydium_accounts.push(AccountMeta::new_readonly(*user_account.key, true));
        raydium_accounts.push(AccountMeta::new(*user_lp_token_account.key, false));
        raydium_accounts.push(AccountMeta::new(*farm_lp_token_account.key, false));
        raydium_accounts.push(AccountMeta::new(*user_reward_token_a_account.key, false));
        raydium_accounts.push(AccountMeta::new(*farm_reward_token_a_account.key, false));
        raydium_accounts.push(AccountMeta::new_readonly(*clock_id.key, false));
        raydium_accounts.push(AccountMeta::new_readonly(*spl_token_id.key, false));
        if dual_rewards {
            raydium_accounts.push(AccountMeta::new(*user_reward_token_b_account.key, false));
            raydium_accounts.push(AccountMeta::new(*farm_reward_token_b_account.key, false));
        }

        let lp_amount = if harvest {
            0
        } else if amount > 0 {
            amount
        } else {
            initial_lp_token_user_balance
        };

        let instruction = Instruction {
            program_id: *farm_program_id.key,
            accounts: raydium_accounts,
            data: RaydiumStake {
                instruction: 1,
                amount: lp_amount,
            }
            .to_vec()?,
        };
        invoke(&instruction, accounts)?;

        account::check_tokens_spent(
            user_lp_token_account,
            initial_lp_token_user_balance,
            lp_amount,
        )?;
        if user_lp_token_account.key != user_reward_token_a_account.key {
            let _ = account::get_balance_increase(
                user_reward_token_a_account,
                initial_token_a_user_balance,
            )?;
        }
        if dual_rewards && user_lp_token_account.key != user_reward_token_b_account.key {
            let _ = account::get_balance_increase(
                user_reward_token_b_account,
                initial_token_b_user_balance,
            )?;
        }
    } else {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    msg!("AmmInstruction::Stake complete");
    Ok(())
}
