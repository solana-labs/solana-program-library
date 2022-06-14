//! Swap tokens with the Raydium pool instruction

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        error::FarmError,
        fund::Fund,
        instruction::amm::AmmInstruction,
        program,
        program::{account, clock},
    },
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        msg,
        program::invoke_signed,
        program_error::ProgramError,
    },
};

pub fn swap(
    fund: &Fund,
    accounts: &[AccountInfo],
    token_a_amount_in: u64,
    token_b_amount_in: u64,
    min_token_amount_out: u64,
) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _admin_account,
        fund_metadata,
        fund_info_account,
        fund_authority,
        router_program_id,
        fund_vault_metadata,
        fund_token_a_account,
        fund_token_b_account,
        pool_program_id,
        pool_coin_token_account,
        pool_pc_token_account,
        spl_token_program,
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
        serum_vault_signer,
        sysvar_account
        ] = accounts
    {
        // validate params and accounts
        msg!("Validate state and accounts");
        let mut fund_info = FundInfo::new(fund_info_account);
        if fund_info.get_liquidation_start_time()? > 0 {
            let curtime = clock::get_time()?;
            let last_trade_time = fund_info.get_last_trade_time()?;
            if last_trade_time > 0 && curtime - last_trade_time < 300 {
                msg!(
                    "Error: Too early for another swap, please retry in {} seconds",
                    300 - curtime - last_trade_time
                );
                return Err(FarmError::TooEarly.into());
            }
        }
        if !program::is_last_instruction(sysvar_account)? {
            msg!("Error: Swap must be the last instruction in the transaction");
            return Err(ProgramError::InvalidArgument);
        }

        if fund_authority.key != &fund.fund_authority {
            msg!("Error: Invalid Fund authority account");
            return Err(ProgramError::Custom(517));
        }

        common::check_unpack_target_vault(
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

        let raydium_accounts = vec![
            AccountMeta::new_readonly(*fund_authority.key, true),
            AccountMeta::new(*fund_token_a_account.key, false),
            AccountMeta::new(*fund_token_b_account.key, false),
            AccountMeta::new_readonly(*pool_program_id.key, false),
            AccountMeta::new(*pool_coin_token_account.key, false),
            AccountMeta::new(*pool_pc_token_account.key, false),
            AccountMeta::new_readonly(*spl_token_program.key, false),
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
            data: AmmInstruction::Swap {
                token_a_amount_in,
                token_b_amount_in,
                min_token_amount_out,
            }
            .to_vec()?,
        };

        invoke_signed(&instruction, accounts, seeds)?;

        msg!(
            "token_a_balance: {}, token_b_balance: {}",
            account::get_token_balance(fund_token_a_account)?,
            account::get_token_balance(fund_token_b_account)?
        );

        // update fund stats
        msg!("Update Fund stats");
        fund_info.update_last_trade_time()
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
