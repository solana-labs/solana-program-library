//! Initialize a new user for a Orca farm instruction

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{fund::Fund, instruction::amm::AmmInstruction},
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        msg,
        program::invoke,
        program_error::ProgramError,
    },
};

pub fn user_init(fund: &Fund, accounts: &[AccountInfo]) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        admin_account,
        fund_metadata,
        fund_info_account,
        fund_authority,
        router_program_id,
        fund_vault_metadata,
        _fund_wallet_account,
        fund_stake_info_account,
        farm_id,
        farm_program_id,
        system_program
        ] = accounts
    {
        // validate accounts
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

        common::check_unpack_target_vault(
            &fund.fund_program_id,
            router_program_id.key,
            fund_metadata.key,
            farm_id.key,
            fund_vault_metadata,
        )?;

        // prepare instruction and call orca router
        let orca_accounts = vec![
            AccountMeta::new(*admin_account.key, true),
            AccountMeta::new_readonly(*fund_authority.key, false),
            AccountMeta::new(*fund_stake_info_account.key, false),
            AccountMeta::new_readonly(*farm_id.key, false),
            AccountMeta::new_readonly(*farm_program_id.key, false),
            AccountMeta::new_readonly(*system_program.key, false),
        ];

        let instruction = Instruction {
            program_id: *router_program_id.key,
            accounts: orca_accounts,
            data: AmmInstruction::UserInit {}.to_vec()?,
        };

        invoke(&instruction, accounts)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
