//! Initialize a new user for a Raydium farm instruction

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        farm::{Farm, FarmRoute},
        fund::Fund,
        id::main_router,
        instruction::amm::AmmInstruction,
        program::account,
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
        farm_metadata,
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

        if farm_metadata.owner != &main_router::id() {
            msg!("Error: Invalid Farm metadata owner");
            return Err(ProgramError::IllegalOwner);
        }
        let farm = account::unpack::<Farm>(farm_metadata, "Farm")?;
        let farm_id = match farm.route {
            FarmRoute::Raydium { farm_id, .. } => farm_id,
            _ => {
                msg!("Error: Unsupported Farm route");
                return Err(ProgramError::Custom(537));
            }
        };

        common::check_unpack_target_vault(
            &fund.fund_program_id,
            router_program_id.key,
            fund_metadata.key,
            &farm_id,
            fund_vault_metadata,
        )?;

        // prepare instruction and call raydium router
        let raydium_accounts = vec![
            AccountMeta::new(*admin_account.key, true),
            AccountMeta::new_readonly(*fund_authority.key, false),
            AccountMeta::new(*fund_stake_info_account.key, false),
            AccountMeta::new_readonly(*farm_metadata.key, false),
            AccountMeta::new_readonly(*system_program.key, false),
        ];

        let instruction = Instruction {
            program_id: *router_program_id.key,
            accounts: raydium_accounts,
            data: AmmInstruction::UserInit {}.to_vec()?,
        };

        invoke(&instruction, accounts)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
