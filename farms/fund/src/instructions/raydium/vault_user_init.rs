//! Init new user in the Raydium Vault instruction

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{fund::Fund, instruction::vault::VaultInstruction},
    solana_program::{
        account_info::AccountInfo,
        entrypoint::ProgramResult,
        instruction::{AccountMeta, Instruction},
        msg,
        program::invoke_signed,
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
        vault_program_id,
        fund_vault_metadata,
        vault_metadata,
        vault_info_account,
        _fund_wallet_account,
        fund_vault_user_account,
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

        let vault_accounts = vec![
            AccountMeta::new(*admin_account.key, true),
            AccountMeta::new_readonly(*vault_metadata.key, false),
            AccountMeta::new(*vault_info_account.key, false),
            AccountMeta::new_readonly(*fund_authority.key, false),
            AccountMeta::new(*fund_vault_user_account.key, false),
            AccountMeta::new_readonly(*system_program.key, false),
        ];

        let instruction = Instruction {
            program_id: *vault_program_id.key,
            accounts: vault_accounts,
            data: VaultInstruction::UserInit {}.to_vec()?,
        };

        invoke_signed(&instruction, accounts, seeds)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
