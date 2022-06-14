//! Unstake LP tokens from a Orca farm instruction

use {
    crate::common,
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

pub fn unstake(fund: &Fund, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _admin_account,
        fund_metadata,
        _fund_info_account,
        fund_authority,
        router_program_id,
        fund_vault_metadata,
        fund_stake_info_account,
        fund_lp_token_account,
        fund_reward_token_account,
        fund_farm_lp_token_account,
        farm_lp_token_mint,
        farm_program_id,
        farm_base_token_vault,
        farm_reward_token_vault,
        spl_token_id,
        farm_id,
        farm_authority
        ] = accounts
    {
        // validate params and accounts
        msg!("Validate state and accounts");
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

        // prepare instruction and call orca router
        let seeds: &[&[&[u8]]] = &[&[
            b"fund_authority",
            fund.name.as_bytes(),
            &[fund.authority_bump],
        ]];

        let orca_accounts = vec![
            AccountMeta::new_readonly(*fund_authority.key, true),
            AccountMeta::new(*fund_stake_info_account.key, false),
            AccountMeta::new(*fund_lp_token_account.key, false),
            AccountMeta::new(*fund_reward_token_account.key, false),
            AccountMeta::new(*fund_farm_lp_token_account.key, false),
            AccountMeta::new(*farm_lp_token_mint.key, false),
            AccountMeta::new_readonly(*farm_program_id.key, false),
            AccountMeta::new(*farm_base_token_vault.key, false),
            AccountMeta::new(*farm_reward_token_vault.key, false),
            AccountMeta::new_readonly(*spl_token_id.key, false),
            AccountMeta::new(*farm_id.key, false),
            AccountMeta::new_readonly(*farm_authority.key, false),
        ];

        let instruction = Instruction {
            program_id: *router_program_id.key,
            accounts: orca_accounts,
            data: AmmInstruction::Unstake { amount }.to_vec()?,
        };

        invoke_signed(&instruction, accounts, seeds)?;

        msg!(
            "reward_balance: {}, farm_lp_token_balance: {}, lp_token_balance: {}",
            account::get_token_balance(fund_reward_token_account)?,
            account::get_token_balance(fund_farm_lp_token_account)?,
            account::get_token_balance(fund_lp_token_account)?
        );

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
