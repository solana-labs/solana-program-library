//! Fund AddVault instruction handler

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        farm::{Farm, FarmRoute},
        fund::{Fund, FundAssetType, FundVault, FundVaultType, DISCRIMINATOR_FUND_VAULT},
        id::main_router,
        pool::{Pool, PoolRoute},
        program::{account, pda},
        token::Token,
        traits::Packed,
        vault::{Vault, VaultStrategy},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn add_vault(
    fund: &Fund,
    accounts: &[AccountInfo],
    target_hash: u64,
    vault_id: u32,
    vault_type: FundVaultType,
) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        admin_account,
        fund_metadata,
        fund_info_account,
        _multisig_account,
        fund_authority,
        _system_program,
        vaults_assets_info,
        fund_vault_metadata,
        target_vault_metadata,
        router_program_id,
        underlying_pool_id,
        underlying_pool_ref,
        underlying_lp_token_metadata
        ] = accounts
    {
        // validate accounts
        msg!("Validate state and accounts");
        let mut fund_info = FundInfo::new(fund_info_account);
        if fund_info.get_liquidation_start_time()? > 0 {
            msg!("Error: Fund is in liquidation state");
            return Err(ProgramError::Custom(516));
        }
        if fund_authority.key != &fund.fund_authority {
            msg!("Error: Invalid Fund authority account");
            return Err(ProgramError::Custom(517));
        }
        if underlying_pool_ref.owner != &main_router::id() {
            msg!("Error: Invalid pool metadata owner");
            return Err(ProgramError::IllegalOwner);
        }
        if underlying_lp_token_metadata.owner != &main_router::id() {
            msg!("Error: Invalid lp token metadata owner");
            return Err(ProgramError::IllegalOwner);
        }
        if target_vault_metadata.owner != &main_router::id() {
            msg!("Error: Invalid target vault metadata owner");
            return Err(ProgramError::IllegalOwner);
        }

        // target_vault_metadata represents different data structure depending on vault_type
        let (vault_name, vault_router, pool_id) = match vault_type {
            FundVaultType::Vault => {
                let vault = account::unpack::<Vault>(target_vault_metadata, "Vault")?;
                if let VaultStrategy::StakeLpCompoundRewards { pool_ref, .. } = vault.strategy {
                    if &pool_ref != underlying_pool_ref.key {
                        msg!("Error: Underlying pool address mismatch");
                        return Err(ProgramError::Custom(520));
                    }
                    (
                        vault.name,
                        vault.vault_program_id,
                        *target_vault_metadata.key,
                    )
                } else {
                    msg!("Error: Vault strategy unsupported");
                    return Err(ProgramError::Custom(521));
                }
            }
            FundVaultType::Pool => {
                let pool = account::unpack::<Pool>(target_vault_metadata, "Pool")?;
                let pool_ammid = match pool.route {
                    PoolRoute::Raydium { amm_id, .. } => amm_id,
                    PoolRoute::Orca { amm_id, .. } => amm_id,
                    _ => {
                        msg!("Error: Unsupported Pool route");
                        return Err(ProgramError::Custom(522));
                    }
                };
                (pool.name, pool.router_program_id, pool_ammid)
            }
            FundVaultType::Farm => {
                let farm = account::unpack::<Farm>(target_vault_metadata, "Farm")?;
                let farm_id = match farm.route {
                    FarmRoute::Raydium { farm_id, .. } => farm_id,
                    FarmRoute::Orca { farm_id, .. } => farm_id,
                    _ => {
                        msg!("Error: Unsupported Farm route");
                        return Err(ProgramError::Custom(522));
                    }
                };
                (farm.name, farm.router_program_id, farm_id)
            }
        };

        if &vault_router != router_program_id.key {
            msg!("Error:Invalit router program id");
            return Err(ProgramError::IncorrectProgramId);
        }
        if &pool_id != underlying_pool_id.key {
            msg!("Error: Invalid underlying pool id");
            return Err(ProgramError::Custom(523));
        }

        let underlying_pool = account::unpack::<Pool>(underlying_pool_ref, "Underlying Pool")?;
        let underlying_lp_token =
            account::unpack::<Token>(underlying_lp_token_metadata, "Underlying LP Token")?;
        if underlying_pool.lp_token_ref.is_none()
            || underlying_lp_token_metadata.key != &underlying_pool.lp_token_ref.unwrap()
        {
            msg!("Error: Underlying LP token address mismatch");
            return Err(ProgramError::Custom(524));
        }

        if account::exists(fund_vault_metadata)? {
            msg!("Error: Vault already initialized");
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        // init vault metadata account
        msg!("Init vault metadata account");
        msg!(
            "vault_name: {}, vault_id: {}, vault_type: {:?}",
            vault_name,
            vault_id,
            vault_type
        );

        let vault_seed_str: &[u8] = match vault_type {
            FundVaultType::Vault => b"fund_vault_info",
            FundVaultType::Pool => b"fund_pool_info",
            FundVaultType::Farm => b"fund_farm_info",
        };
        let vault_seeds = &[vault_seed_str, vault_name.as_bytes(), fund.name.as_bytes()];
        let bump = pda::init_system_account(
            admin_account,
            fund_vault_metadata,
            &fund.fund_program_id,
            &fund.fund_program_id,
            vault_seeds,
            FundVault::LEN,
        )?;

        let vault = FundVault {
            discriminator: DISCRIMINATOR_FUND_VAULT,
            fund_ref: *fund_metadata.key,
            vault_id,
            vault_type,
            vault_ref: *target_vault_metadata.key,
            router_program_id: *router_program_id.key,
            underlying_pool_id: *underlying_pool_id.key,
            underlying_pool_ref: *underlying_pool_ref.key,
            underlying_lp_token_mint: underlying_lp_token.mint,
            lp_balance: 0,
            balance_update_time: 0,
            bump,
        };
        vault.pack(*fund_vault_metadata.try_borrow_mut_data()?)?;

        // since each non-farm vault must be counted in update_assets_with_vault()
        // we reset fund_assets stats
        if vault_type != FundVaultType::Farm {
            // update assets tracking account
            msg!("Update Fund assets account");
            let mut fund_assets = common::check_and_get_fund_assets_account(
                fund,
                vaults_assets_info,
                FundAssetType::Vault,
            )?;
            fund_assets.current_hash = 0;
            fund_assets.target_hash = target_hash;
            fund_assets.current_assets_usd = 0.0;
            fund_assets.cycle_start_time = 0;
            fund_assets.cycle_end_time = 0;
            fund_assets.pack(*vaults_assets_info.try_borrow_mut_data()?)?;
        }

        // update fund stats
        msg!("Update Fund stats");
        fund_info.update_admin_action_time()
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
