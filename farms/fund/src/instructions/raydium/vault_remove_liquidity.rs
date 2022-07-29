//! Remove liquidity from the Raydium Vault instruction

use {
    crate::common,
    solana_farm_sdk::{fund::Fund, instruction::vault::VaultInstruction, program::account},
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        msg,
        program::invoke_signed,
        program_error::ProgramError,
    },
};

pub fn remove_liquidity(fund: &Fund, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _admin_account,
        fund_metadata,
        _fund_info_account,
        fund_authority,
        vault_program_id,
        fund_vault_metadata,
        vault_metadata,
        vault_info_account,
        vault_authority,
        spl_token_program,
        fund_vault_user_account,
        fund_token_a_custody,
        fund_token_b_custody,
        vault_lp_token_custody,
        pool_program_id,
        pool_withdraw_queue,
        pool_temp_lp_token_account,
        pool_coin_token_account,
        pool_pc_token_account,
        lp_token_mint,
        amm_id,
        amm_authority,
        amm_open_orders,
        amm_target,
        serum_market,
        serum_program_id,
        serum_bids,
        serum_asks,
        serum_event_queue,
        serum_coin_vault_account,
        serum_pc_vault_account,
        serum_vault_signer
        ] = accounts
    {
        // validate accounts
        msg!("Validate state and accounts");
        if fund_authority.key != &fund.fund_authority {
            msg!("Error: Invalid Fund authority account");
            return Err(ProgramError::Custom(517));
        }

        let vault = common::check_unpack_target_vault(
            &fund.fund_program_id,
            vault_program_id.key,
            fund_metadata.key,
            vault_metadata.key,
            fund_vault_metadata,
        )?;

        // prepare instruction and call vault program
        let seeds: &[&[&[u8]]] = &[&[
            b"fund_authority",
            fund.name.as_bytes(),
            &[fund.authority_bump],
        ]];

        let initial_lp_balance = account::get_token_balance(vault_lp_token_custody)?;

        let vault_accounts = vec![
            AccountMeta::new_readonly(*fund_authority.key, true),
            AccountMeta::new_readonly(*vault_metadata.key, false),
            AccountMeta::new(*vault_info_account.key, false),
            AccountMeta::new_readonly(*vault_authority.key, false),
            AccountMeta::new_readonly(*spl_token_program.key, false),
            AccountMeta::new(*fund_vault_user_account.key, false),
            AccountMeta::new(*fund_token_a_custody.key, false),
            AccountMeta::new(*fund_token_b_custody.key, false),
            AccountMeta::new(*vault_lp_token_custody.key, false),
            AccountMeta::new_readonly(*pool_program_id.key, false),
            AccountMeta::new(*pool_withdraw_queue.key, false),
            AccountMeta::new(*pool_temp_lp_token_account.key, false),
            AccountMeta::new(*pool_coin_token_account.key, false),
            AccountMeta::new(*pool_pc_token_account.key, false),
            AccountMeta::new(*lp_token_mint.key, false),
            AccountMeta::new(*amm_id.key, false),
            AccountMeta::new_readonly(*amm_authority.key, false),
            AccountMeta::new(*amm_open_orders.key, false),
            AccountMeta::new(*amm_target.key, false),
            AccountMeta::new(*serum_market.key, false),
            AccountMeta::new_readonly(*serum_program_id.key, false),
            AccountMeta::new(*serum_bids.key, false),
            AccountMeta::new(*serum_asks.key, false),
            AccountMeta::new(*serum_event_queue.key, false),
            AccountMeta::new(*serum_coin_vault_account.key, false),
            AccountMeta::new(*serum_pc_vault_account.key, false),
            AccountMeta::new_readonly(*serum_vault_signer.key, false),
        ];

        let instruction = Instruction {
            program_id: *vault_program_id.key,
            accounts: vault_accounts,
            data: VaultInstruction::RemoveLiquidity { amount }.to_vec()?,
        };

        invoke_signed(&instruction, accounts, seeds)?;

        // update stats
        msg!("Update vault balance");
        let lp_removed = account::get_balance_decrease(vault_lp_token_custody, initial_lp_balance)?;
        common::decrease_vault_balance(fund_vault_metadata, &vault, lp_removed)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
