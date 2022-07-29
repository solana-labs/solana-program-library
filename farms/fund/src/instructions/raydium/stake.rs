//! Stake LP tokens to a Raydium farm instruction

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{fund::Fund, instruction::amm::AmmInstruction, program::account},
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        msg,
        program::invoke_signed,
        program_error::ProgramError,
    },
};

pub fn stake(fund: &Fund, accounts: &[AccountInfo], amount: u64, harvest: bool) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _admin_account,
        fund_metadata,
        fund_info_account,
        fund_authority,
        router_program_id,
        fund_vault_metadata,
        fund_stake_info_account,
        fund_lp_token_account,
        fund_first_reward_token_account,
        fund_second_reward_token_account,
        farm_program_id,
        farm_lp_token_account,
        farm_first_reward_token_account,
        farm_second_reward_token_account,
        clock_id,
        spl_token_id,
        farm_id,
        farm_authority
        ] = accounts
    {
        // validate params and accounts
        msg!("Validate state and accounts");
        if !harvest && FundInfo::new(fund_info_account).get_liquidation_start_time()? > 0 {
            msg!("Error: Fund is in liquidation state");
            return Err(ProgramError::Custom(516));
        }
        if fund_authority.key != &fund.fund_authority {
            msg!("Error: Invalid Fund authority account");
            return Err(ProgramError::Custom(517));
        }
        if account::is_empty(fund_stake_info_account)? {
            msg!("Error: Fund stake info account must be initialized first");
            return Err(ProgramError::UninitializedAccount);
        }

        common::check_unpack_target_vault(
            &fund.fund_program_id,
            router_program_id.key,
            fund_metadata.key,
            farm_id.key,
            fund_vault_metadata,
        )?;

        // prepare instruction and call raydium router
        let seeds: &[&[&[u8]]] = &[&[
            b"fund_authority",
            fund.name.as_bytes(),
            &[fund.authority_bump],
        ]];

        let raydium_accounts = vec![
            AccountMeta::new_readonly(*fund_authority.key, true),
            AccountMeta::new(*fund_stake_info_account.key, false),
            AccountMeta::new(*fund_lp_token_account.key, false),
            AccountMeta::new(*fund_first_reward_token_account.key, false),
            AccountMeta::new(*fund_second_reward_token_account.key, false),
            AccountMeta::new_readonly(*farm_program_id.key, false),
            AccountMeta::new(*farm_lp_token_account.key, false),
            AccountMeta::new(*farm_first_reward_token_account.key, false),
            AccountMeta::new(*farm_second_reward_token_account.key, false),
            AccountMeta::new_readonly(*clock_id.key, false),
            AccountMeta::new_readonly(*spl_token_id.key, false),
            AccountMeta::new(*farm_id.key, false),
            AccountMeta::new_readonly(*farm_authority.key, false),
        ];

        let instruction = Instruction {
            program_id: *router_program_id.key,
            accounts: raydium_accounts,
            data: if harvest {
                AmmInstruction::Harvest.to_vec()?
            } else {
                AmmInstruction::Stake { amount }.to_vec()?
            },
        };

        invoke_signed(&instruction, accounts, seeds)?;

        msg!(
            "reward_a_balance: {}, reward_b_balance: {}, lp_token_balance: {}",
            account::get_token_balance(fund_first_reward_token_account)?,
            account::get_token_balance(fund_second_reward_token_account)?,
            account::get_token_balance(fund_lp_token_account)?
        );

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
