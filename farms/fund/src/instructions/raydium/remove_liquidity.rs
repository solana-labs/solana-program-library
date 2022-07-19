//! Remove liquidity from the Raydium pool instruction

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

pub fn remove_liquidity(fund: &Fund, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _admin_account,
        fund_metadata,
        _fund_info_account,
        fund_authority,
        router_program_id,
        fund_vault_metadata,
        fund_token_a_account,
        fund_token_b_account,
        fund_lp_token_account,
        pool_program_id,
        pool_withdraw_queue,
        pool_temp_lp_token_account,
        pool_coin_token_account,
        pool_pc_token_account,
        lp_token_mint,
        spl_token_id,
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
        // validate params and accounts
        msg!("Validate state and accounts");
        if fund_authority.key != &fund.fund_authority {
            msg!("Error: Invalid Fund authority account");
            return Err(ProgramError::Custom(517));
        }

        let vault = common::check_unpack_target_vault(
            &fund.fund_program_id,
            router_program_id.key,
            fund_metadata.key,
            amm_id.key,
            fund_vault_metadata,
        )?;

        // prepare instruction and call raydium router
        let seeds: &[&[&[u8]]] = &[&[
            b"fund_authority",
            fund.name.as_bytes(),
            &[fund.authority_bump],
        ]];

        let initial_lp_balance = account::get_token_balance(fund_lp_token_account)?;

        let raydium_accounts = vec![
            AccountMeta::new_readonly(*fund_authority.key, true),
            AccountMeta::new(*fund_token_a_account.key, false),
            AccountMeta::new(*fund_token_b_account.key, false),
            AccountMeta::new(*fund_lp_token_account.key, false),
            AccountMeta::new_readonly(*pool_program_id.key, false),
            AccountMeta::new(*pool_withdraw_queue.key, false),
            AccountMeta::new(*pool_temp_lp_token_account.key, false),
            AccountMeta::new(*pool_coin_token_account.key, false),
            AccountMeta::new(*pool_pc_token_account.key, false),
            AccountMeta::new(*lp_token_mint.key, false),
            AccountMeta::new_readonly(*spl_token_id.key, false),
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
            program_id: *router_program_id.key,
            accounts: raydium_accounts,
            data: AmmInstruction::RemoveLiquidity { amount }.to_vec()?,
        };

        invoke_signed(&instruction, accounts, seeds)?;

        // update stats
        msg!("Update vault balance");
        let lp_removed = account::get_balance_decrease(fund_lp_token_account, initial_lp_balance)?;
        msg!(
            "token_a_balance: {}, token_b_balance: {}, lp_removed: {}",
            account::get_token_balance(fund_token_a_account)?,
            account::get_token_balance(fund_token_b_account)?,
            lp_removed
        );
        common::decrease_vault_balance(fund_vault_metadata, &vault, lp_removed)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
