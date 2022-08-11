//! Add liquidity to the Orca pool instruction

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

pub fn add_liquidity(
    fund: &Fund,
    accounts: &[AccountInfo],
    max_token_a_amount: u64,
    max_token_b_amount: u64,
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
        fund_lp_token_account,
        pool_program_id,
        pool_token_a_account,
        pool_token_b_account,
        lp_token_mint,
        spl_token_id,
        amm_id,
        amm_authority
        ] = accounts
    {
        // validate params and accounts
        msg!("Validate state and accounts");
        let fund_info = FundInfo::new(fund_info_account);
        if fund_info.get_liquidation_start_time()? > 0 {
            msg!("Error: Fund is in liquidation state");
            return Err(ProgramError::Custom(516));
        }
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

        // prepare instruction and call orca router
        let seeds: &[&[&[u8]]] = &[&[
            b"fund_authority",
            fund.name.as_bytes(),
            &[fund.authority_bump],
        ]];

        let initial_lp_balance = account::get_token_balance(fund_lp_token_account)?;

        let orca_accounts = vec![
            AccountMeta::new_readonly(*fund_authority.key, true),
            AccountMeta::new(*fund_token_a_account.key, false),
            AccountMeta::new(*fund_token_b_account.key, false),
            AccountMeta::new(*fund_lp_token_account.key, false),
            AccountMeta::new_readonly(*pool_program_id.key, false),
            AccountMeta::new(*pool_token_a_account.key, false),
            AccountMeta::new(*pool_token_b_account.key, false),
            AccountMeta::new(*lp_token_mint.key, false),
            AccountMeta::new_readonly(*spl_token_id.key, false),
            AccountMeta::new_readonly(*amm_id.key, false),
            AccountMeta::new_readonly(*amm_authority.key, false),
        ];

        let instruction = Instruction {
            program_id: *router_program_id.key,
            accounts: orca_accounts,
            data: AmmInstruction::AddLiquidity {
                max_token_a_amount,
                max_token_b_amount,
            }
            .to_vec()?,
        };

        invoke_signed(&instruction, accounts, seeds)?;

        // update stats
        msg!("Update vault balance");
        let lp_received = account::get_balance_increase(fund_lp_token_account, initial_lp_balance)?;
        msg!(
            "token_a_balance: {}, token_b_balance: {}, lp_received: {}",
            account::get_token_balance(fund_token_a_account)?,
            account::get_token_balance(fund_token_b_account)?,
            lp_received
        );
        common::increase_vault_balance(fund_vault_metadata, &vault, lp_received)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
