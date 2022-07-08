//! Solana Farm Client Fund Instructions

use {
    crate::error::FarmClientError,
    solana_farm_sdk::{
        farm::FarmRoute,
        fund::{
            FundAssetType, FundAssetsTrackingConfig, FundCustodyType, FundSchedule, FundVaultType,
        },
        id::zero,
        instruction::fund::FundInstruction,
        math,
        pool::PoolRoute,
        program::multisig::Multisig,
        string::str_to_as64,
        token::OracleType,
        vault::VaultStrategy,
    },
    solana_sdk::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        system_program, sysvar,
    },
};

use super::FarmClient;

impl FarmClient {
    /// Creates a new Fund Init Instruction
    pub fn new_instruction_init_fund(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        step: u64,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let fund_token = self.get_token_by_ref(&fund.fund_token_ref)?;

        // fill in accounts and instruction data
        let data = FundInstruction::Init { step }.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
            AccountMeta::new(fund.fund_authority, false),
            AccountMeta::new_readonly(fund.fund_program_id, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new(fund_token.mint, false),
            AccountMeta::new_readonly(fund.fund_token_ref, false),
            AccountMeta::new(fund.vaults_assets_info, false),
            AccountMeta::new(fund.custodies_assets_info, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for initializing a new User for the Fund
    pub fn new_instruction_user_init_fund(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_info_account = self.get_fund_user_info_account(wallet_address, fund_name)?;
        let user_requests_account =
            self.get_fund_user_requests_account(wallet_address, fund_name, token_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::UserInit.to_vec()?;
        let accounts = vec![
            AccountMeta::new(*wallet_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(*wallet_address, false),
            AccountMeta::new(user_info_account, false),
            AccountMeta::new(user_requests_account, false),
            AccountMeta::new_readonly(token_ref, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new instruction for initializing Fund's multisig with a new set of signers
    pub fn new_instruction_set_fund_admins(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        admin_signers: &[Pubkey],
        min_signatures: u8,
    ) -> Result<Instruction, FarmClientError> {
        if admin_signers.is_empty() || min_signatures == 0 {
            return Err(FarmClientError::ValueError(
                "At least one signer is required".to_string(),
            ));
        } else if min_signatures as usize > admin_signers.len()
            || admin_signers.len() > Multisig::MAX_SIGNERS
        {
            return Err(FarmClientError::ValueError(
                "Invalid number of signatures".to_string(),
            ));
        }

        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: fund.fund_program_id,
            data: FundInstruction::SetAdminSigners { min_signatures }.to_vec()?,
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new_readonly(fund_ref, false),
                AccountMeta::new(fund.info_account, false),
                AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
                AccountMeta::new(self.get_fund_multisig_account(fund_name)?, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        for key in admin_signers {
            inst.accounts.push(AccountMeta::new_readonly(*key, false));
        }

        Ok(inst)
    }

    /// Creates a new instruction for removing Fund's multisig
    pub fn new_instruction_remove_fund_multisig(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // fill in accounts and instruction data
        let inst = Instruction {
            program_id: fund.fund_program_id,
            data: FundInstruction::RemoveMultisig.to_vec()?,
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new_readonly(fund_ref, false),
                AccountMeta::new(fund.info_account, false),
                AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
                AccountMeta::new(self.get_fund_multisig_account(fund_name)?, false),
            ],
        };

        Ok(inst)
    }

    /// Creates a new set fund assets tracking config Instruction
    pub fn new_instruction_set_fund_assets_tracking_config(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        config: &FundAssetsTrackingConfig,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::SetAssetsTrackingConfig { config: *config }.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for adding a new custody to the Fund
    pub fn new_instruction_add_fund_custody(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        custody_type: FundCustodyType,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;

        // get custodies
        let custodies = self.get_fund_custodies(fund_name)?;
        let custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, custody_type)?;
        let fund_assets_account =
            self.get_fund_assets_account(fund_name, FundAssetType::Custody)?;
        let custody_token_account =
            self.get_fund_custody_token_account(fund_name, token_name, custody_type)?;
        let custody_fees_token_account =
            self.get_fund_custody_fees_token_account(fund_name, token_name, custody_type)?;

        // instruction params
        let custody_id = if custodies.is_empty() {
            0
        } else if custodies.last().unwrap().custody_id < u32::MAX {
            custodies.last().unwrap().custody_id + 1
        } else {
            return Err(FarmClientError::ValueError(
                "Number of custodies are over the limit".to_string(),
            ));
        };

        let current_hash = self
            .get_fund_assets(fund_name, FundAssetType::Custody)?
            .target_hash;

        let target_hash = if FarmClient::is_liquidity_token(token_name) {
            current_hash
        } else {
            math::hash_address(current_hash, &custody_token_account)
        };

        // fill in accounts and instruction data
        let data = FundInstruction::AddCustody {
            target_hash,
            custody_id,
            custody_type,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
            AccountMeta::new(self.get_fund_multisig_account(fund_name)?, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new(fund_assets_account, false),
            AccountMeta::new(custody_token_account, false),
            AccountMeta::new(custody_fees_token_account, false),
            AccountMeta::new(custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
            AccountMeta::new(token.mint, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for removing the custody from the Fund
    pub fn new_instruction_remove_fund_custody(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        custody_type: FundCustodyType,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token_ref = self.get_token_ref(token_name)?;

        // get custodies
        let custodies = self.get_fund_custodies(fund_name)?;
        if custodies.is_empty() {
            return Err(FarmClientError::ValueError(
                "No active custodies found".to_string(),
            ));
        }
        let custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, custody_type)?;
        let fund_assets_account =
            self.get_fund_assets_account(fund_name, FundAssetType::Custody)?;
        let custody_token_account =
            self.get_fund_custody_token_account(fund_name, token_name, custody_type)?;
        let custody_fees_token_account =
            self.get_fund_custody_fees_token_account(fund_name, token_name, custody_type)?;

        // instruction params
        let mut target_hash = 0;
        for custody in custodies {
            if custody.address != custody_token_account && !custody.is_vault_token {
                target_hash = math::hash_address(target_hash, &custody.address);
            }
        }

        // fill in accounts and instruction data
        let data = FundInstruction::RemoveCustody {
            target_hash,
            custody_type,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
            AccountMeta::new(self.get_fund_multisig_account(fund_name)?, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(fund_assets_account, false),
            AccountMeta::new(custody_token_account, false),
            AccountMeta::new(custody_fees_token_account, false),
            AccountMeta::new(custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for adding a new Vault to the Fund
    pub fn new_instruction_add_fund_vault(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        vault_name: &str,
        vault_type: FundVaultType,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // get vaults
        let vaults = self.get_fund_vaults(fund_name)?;
        let fund_vault_metadata = self.get_fund_vault_account(fund_name, vault_name, vault_type)?;
        let fund_assets_account = self.get_fund_assets_account(fund_name, FundAssetType::Vault)?;
        let target_vault_metadata = match vault_type {
            FundVaultType::Vault => self.get_vault_ref(vault_name)?,
            FundVaultType::Pool => self.get_pool_ref(vault_name)?,
            FundVaultType::Farm => self.get_farm_ref(vault_name)?,
        };
        let underlying_pool_ref = match vault_type {
            FundVaultType::Vault => {
                let vault = self.get_vault(vault_name)?;
                match vault.strategy {
                    VaultStrategy::StakeLpCompoundRewards { pool_ref, .. } => pool_ref,
                    _ => unreachable!(),
                }
            }
            FundVaultType::Farm => {
                let farm = self.get_farm(vault_name)?;
                let lp_token = self.get_token_by_ref(&farm.lp_token_ref.ok_or_else(|| {
                    FarmClientError::ValueError("Farms w/o LP tokens are not supported".to_string())
                })?)?;
                let pools = self.find_pools_with_lp(&lp_token.name)?;
                if pools.is_empty() {
                    return Err(FarmClientError::RecordNotFound(format!(
                        "Pools with LP token {}",
                        lp_token.name
                    )));
                }
                self.get_pool_ref(&pools[0].name)?
            }
            FundVaultType::Pool => target_vault_metadata,
        };

        // instruction params
        let vault_id = if vaults.is_empty() {
            0
        } else if vaults.last().unwrap().vault_id < u32::MAX {
            vaults.last().unwrap().vault_id + 1
        } else {
            return Err(FarmClientError::ValueError(
                "Number of vaults are over the limit".to_string(),
            ));
        };

        let current_hash = self
            .get_fund_assets(fund_name, FundAssetType::Vault)?
            .target_hash;

        let target_hash = if vault_type == FundVaultType::Farm {
            current_hash
        } else {
            math::hash_address(current_hash, &target_vault_metadata)
        };

        // fill in accounts and instruction data
        let data = FundInstruction::AddVault {
            target_hash,
            vault_id,
            vault_type,
        }
        .to_vec()?;

        let (router_program_id, underlying_pool_id, underlying_lp_token_metadata) = match vault_type
        {
            FundVaultType::Pool => {
                let pool = self.get_pool(vault_name)?;
                let pool_ammid = match pool.route {
                    PoolRoute::Raydium { amm_id, .. } => amm_id,
                    PoolRoute::Saber { swap_account, .. } => swap_account,
                    PoolRoute::Orca { amm_id, .. } => amm_id,
                };
                (
                    pool.router_program_id,
                    pool_ammid,
                    pool.lp_token_ref.ok_or_else(|| {
                        FarmClientError::ValueError(
                            "Pools w/o LP tokens are not supported".to_string(),
                        )
                    })?,
                )
            }
            FundVaultType::Farm => {
                let farm = self.get_farm(vault_name)?;
                let farm_id = match farm.route {
                    FarmRoute::Raydium { farm_id, .. } => farm_id,
                    FarmRoute::Saber { quarry, .. } => quarry,
                    FarmRoute::Orca { farm_id, .. } => farm_id,
                };
                (
                    farm.router_program_id,
                    farm_id,
                    farm.lp_token_ref.ok_or_else(|| {
                        FarmClientError::ValueError(
                            "Farms w/o LP tokens are not supported".to_string(),
                        )
                    })?,
                )
            }
            FundVaultType::Vault => {
                let vault = self.get_vault(vault_name)?;
                let pool = self.get_pool_by_ref(&underlying_pool_ref)?;

                (
                    vault.vault_program_id,
                    target_vault_metadata,
                    pool.lp_token_ref.ok_or_else(|| {
                        FarmClientError::ValueError(
                            "Underlying Pools w/o LP tokens are not supported".to_string(),
                        )
                    })?,
                )
            }
        };

        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(fund_assets_account, false),
            AccountMeta::new(fund_vault_metadata, false),
            AccountMeta::new_readonly(target_vault_metadata, false),
            AccountMeta::new_readonly(router_program_id, false),
            AccountMeta::new_readonly(underlying_pool_id, false),
            AccountMeta::new_readonly(underlying_pool_ref, false),
            AccountMeta::new_readonly(underlying_lp_token_metadata, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for removing the Vault from the Fund
    pub fn new_instruction_remove_fund_vault(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        vault_name: &str,
        vault_type: FundVaultType,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // get vaults
        let vaults = self.get_fund_vaults(fund_name)?;
        if vaults.is_empty() {
            return Err(FarmClientError::ValueError(
                "No active vaults found".to_string(),
            ));
        }
        let vault_metadata = self.get_fund_vault_account(fund_name, vault_name, vault_type)?;
        let fund_assets_account = self.get_fund_assets_account(fund_name, FundAssetType::Vault)?;
        let vault_info = match vault_type {
            FundVaultType::Vault => self.get_vault_ref(vault_name)?,
            FundVaultType::Pool => self.get_pool_ref(vault_name)?,
            FundVaultType::Farm => self.get_farm_ref(vault_name)?,
        };

        // instruction params
        let mut target_hash = 0;
        for vault in vaults {
            if vault.vault_ref != vault_info && vault.vault_type != FundVaultType::Farm {
                target_hash = math::hash_address(target_hash, &vault.vault_ref);
            }
        }

        // fill in accounts and instruction data
        let data = FundInstruction::RemoveVault {
            target_hash,
            vault_type,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(fund_assets_account, false),
            AccountMeta::new(vault_metadata, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new set deposit schedule Instruction
    pub fn new_instruction_set_fund_deposit_schedule(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        schedule: &FundSchedule,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::SetDepositSchedule {
            schedule: *schedule,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for disabling deposits to the Fund
    pub fn new_instruction_disable_deposits_fund(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::DisableDeposits.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for requesting deposit to the Fund
    pub fn new_instruction_request_deposit_fund(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        if ui_amount < 0.0 {
            return Err(FarmClientError::ValueError(format!(
                "Invalid deposit amount {} specified for Fund {}: Must be greater or equal to zero.",
                ui_amount, fund_name
            )));
        }
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let fund_token = self.get_token_by_ref(&fund.fund_token_ref)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_info_account = self.get_fund_user_info_account(wallet_address, fund_name)?;
        let user_requests_account =
            self.get_fund_user_requests_account(wallet_address, fund_name, token_name)?;
        let user_deposit_token_account =
            self.get_associated_token_address(wallet_address, token.name.as_str())?;
        let user_fund_token_account =
            self.get_associated_token_address(wallet_address, fund_token.name.as_str())?;
        let custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::DepositWithdraw)?;
        let custody_token_account = self.get_fund_custody_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let custody_fees_token_account = self.get_fund_custody_fees_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let (_, oracle_account) = self.get_oracle(token_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::RequestDeposit {
            amount: self.to_token_amount(ui_amount, &token)?,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(fund_token.mint, false),
            AccountMeta::new(user_info_account, false),
            AccountMeta::new(user_requests_account, false),
            AccountMeta::new(user_deposit_token_account, false),
            AccountMeta::new(user_fund_token_account, false),
            AccountMeta::new(custody_token_account, false),
            AccountMeta::new(custody_fees_token_account, false),
            AccountMeta::new_readonly(custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
            AccountMeta::new_readonly(oracle_account.unwrap_or_else(zero::id), false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for canceling pending deposit to the Fund
    pub fn new_instruction_cancel_deposit_fund(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_requests_account =
            self.get_fund_user_requests_account(wallet_address, fund_name, token_name)?;
        let user_deposit_token_account =
            self.get_associated_token_address(wallet_address, token.name.as_str())?;

        // fill in accounts and instruction data
        let data = FundInstruction::CancelDeposit.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(user_requests_account, false),
            AccountMeta::new(user_deposit_token_account, false),
            AccountMeta::new_readonly(token_ref, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for approving deposit to the Fund
    pub fn new_instruction_approve_deposit_fund(
        &self,
        admin_address: &Pubkey,
        user_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        if ui_amount < 0.0 {
            return Err(FarmClientError::ValueError(format!(
                "Invalid approve amount {} specified for Fund {}: Must be greater or equal to zero.",
                ui_amount, fund_name
            )));
        }
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let fund_token = self.get_token_by_ref(&fund.fund_token_ref)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_info_account = self.get_fund_user_info_account(user_address, fund_name)?;
        let user_requests_account =
            self.get_fund_user_requests_account(user_address, fund_name, token_name)?;
        let user_deposit_token_account =
            self.get_associated_token_address(user_address, token.name.as_str())?;
        let user_fund_token_account =
            self.get_associated_token_address(user_address, fund_token.name.as_str())?;
        let custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::DepositWithdraw)?;
        let custody_token_account = self.get_fund_custody_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let custody_fees_token_account = self.get_fund_custody_fees_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let (_, oracle_account) = self.get_oracle(token_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::ApproveDeposit {
            amount: self.to_token_amount(ui_amount, &token)?,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(fund_token.mint, false),
            AccountMeta::new_readonly(*user_address, false),
            AccountMeta::new(user_info_account, false),
            AccountMeta::new(user_requests_account, false),
            AccountMeta::new(user_deposit_token_account, false),
            AccountMeta::new(user_fund_token_account, false),
            AccountMeta::new(custody_token_account, false),
            AccountMeta::new(custody_fees_token_account, false),
            AccountMeta::new_readonly(custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
            AccountMeta::new_readonly(oracle_account.unwrap_or_else(zero::id), false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for denying deposit to the Fund
    pub fn new_instruction_deny_deposit_fund(
        &self,
        admin_address: &Pubkey,
        user_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        deny_reason: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_requests_account =
            self.get_fund_user_requests_account(user_address, fund_name, token_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::DenyDeposit {
            deny_reason: str_to_as64(deny_reason)?,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
            AccountMeta::new_readonly(*user_address, false),
            AccountMeta::new(user_requests_account, false),
            AccountMeta::new_readonly(token_ref, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new set withdrawal schedule Instruction
    pub fn new_instruction_set_fund_withdrawal_schedule(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        schedule: &FundSchedule,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::SetWithdrawalSchedule {
            schedule: *schedule,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for disabling withdrawals from the Fund
    pub fn new_instruction_disable_withdrawals_fund(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::DisableWithdrawals.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for requesting withdrawal from the Fund
    pub fn new_instruction_request_withdrawal_fund(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        if ui_amount < 0.0 {
            return Err(FarmClientError::ValueError(format!(
                "Invalid withdrawal amount {} specified for Fund {}: Must be greater or equal to zero.",
                ui_amount, fund_name
            )));
        }
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let fund_token = self.get_token_by_ref(&fund.fund_token_ref)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_info_account = self.get_fund_user_info_account(wallet_address, fund_name)?;
        let user_requests_account =
            self.get_fund_user_requests_account(wallet_address, fund_name, token_name)?;
        let user_withdrawal_token_account =
            self.get_associated_token_address(wallet_address, token.name.as_str())?;
        let user_fund_token_account =
            self.get_associated_token_address(wallet_address, fund_token.name.as_str())?;
        let custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::DepositWithdraw)?;
        let custody_token_account = self.get_fund_custody_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let custody_fees_token_account = self.get_fund_custody_fees_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let (_, oracle_account) = self.get_oracle(token_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::RequestWithdrawal {
            amount: self.to_token_amount(ui_amount, &fund_token)?,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(fund_token.mint, false),
            AccountMeta::new(user_info_account, false),
            AccountMeta::new(user_requests_account, false),
            AccountMeta::new(user_withdrawal_token_account, false),
            AccountMeta::new(user_fund_token_account, false),
            AccountMeta::new(custody_token_account, false),
            AccountMeta::new(custody_fees_token_account, false),
            AccountMeta::new_readonly(custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
            AccountMeta::new_readonly(oracle_account.unwrap_or_else(zero::id), false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for canceling pending withdrawal from the Fund
    pub fn new_instruction_cancel_withdrawal_fund(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_requests_account =
            self.get_fund_user_requests_account(wallet_address, fund_name, token_name)?;
        let user_withdrawal_token_account =
            self.get_associated_token_address(wallet_address, token.name.as_str())?;

        // fill in accounts and instruction data
        let data = FundInstruction::CancelWithdrawal.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(user_requests_account, false),
            AccountMeta::new(user_withdrawal_token_account, false),
            AccountMeta::new_readonly(token_ref, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for approving withdrawal from the Fund
    pub fn new_instruction_approve_withdrawal_fund(
        &self,
        admin_address: &Pubkey,
        user_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        if ui_amount < 0.0 {
            return Err(FarmClientError::ValueError(format!(
                "Invalid approve amount {} specified for Fund {}: Must be greater or equal to zero.",
                ui_amount, fund_name
            )));
        }
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let fund_token = self.get_token_by_ref(&fund.fund_token_ref)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_info_account = self.get_fund_user_info_account(user_address, fund_name)?;
        let user_requests_account =
            self.get_fund_user_requests_account(user_address, fund_name, token_name)?;
        let user_withdrawal_token_account =
            self.get_associated_token_address(user_address, token.name.as_str())?;
        let user_fund_token_account =
            self.get_associated_token_address(user_address, fund_token.name.as_str())?;
        let custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::DepositWithdraw)?;
        let custody_token_account = self.get_fund_custody_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let custody_fees_token_account = self.get_fund_custody_fees_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let (_, oracle_account) = self.get_oracle(token_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::ApproveWithdrawal {
            amount: self.to_token_amount(ui_amount, &fund_token)?,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(fund_token.mint, false),
            AccountMeta::new_readonly(*user_address, false),
            AccountMeta::new(user_info_account, false),
            AccountMeta::new(user_requests_account, false),
            AccountMeta::new(user_withdrawal_token_account, false),
            AccountMeta::new(user_fund_token_account, false),
            AccountMeta::new(custody_token_account, false),
            AccountMeta::new(custody_fees_token_account, false),
            AccountMeta::new_readonly(custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
            AccountMeta::new_readonly(oracle_account.unwrap_or_else(zero::id), false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for denying withdrawal from the Fund
    pub fn new_instruction_deny_withdrawal_fund(
        &self,
        admin_address: &Pubkey,
        user_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        deny_reason: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let user_requests_account =
            self.get_fund_user_requests_account(user_address, fund_name, token_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::DenyWithdrawal {
            deny_reason: str_to_as64(deny_reason)?,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
            AccountMeta::new_readonly(*user_address, false),
            AccountMeta::new(user_requests_account, false),
            AccountMeta::new_readonly(token_ref, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for moving deposited assets to the Fund
    pub fn new_instruction_lock_assets_fund(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        if ui_amount < 0.0 {
            return Err(FarmClientError::ValueError(format!(
                "Invalid lock amount {} specified for Fund {}: Must be greater or equal to zero.",
                ui_amount, fund_name
            )));
        }
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let wd_custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::DepositWithdraw)?;
        let wd_custody_token_account = self.get_fund_custody_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let trading_custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::Trading)?;
        let trading_custody_token_account =
            self.get_fund_custody_token_account(fund_name, token_name, FundCustodyType::Trading)?;

        // fill in accounts and instruction data
        let data = FundInstruction::LockAssets {
            amount: self.to_token_amount(ui_amount, &token)?,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(wd_custody_token_account, false),
            AccountMeta::new(wd_custody_metadata, false),
            AccountMeta::new(trading_custody_token_account, false),
            AccountMeta::new(trading_custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for releasing assets from the Fund to Deposit/Withdraw custody
    pub fn new_instruction_unlock_assets_fund(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        if ui_amount < 0.0 {
            return Err(FarmClientError::ValueError(format!(
                "Invalid unlock amount {} specified for Fund {}: Must be greater or equal to zero.",
                ui_amount, fund_name
            )));
        }
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let token = self.get_token(token_name)?;
        let token_ref = self.get_token_ref(token_name)?;
        let wd_custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::DepositWithdraw)?;
        let wd_custody_token_account = self.get_fund_custody_token_account(
            fund_name,
            token_name,
            FundCustodyType::DepositWithdraw,
        )?;
        let trading_custody_metadata =
            self.get_fund_custody_account(fund_name, token_name, FundCustodyType::Trading)?;
        let trading_custody_token_account =
            self.get_fund_custody_token_account(fund_name, token_name, FundCustodyType::Trading)?;

        // fill in accounts and instruction data
        let data = FundInstruction::UnlockAssets {
            amount: self.to_token_amount(ui_amount, &token)?,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
            AccountMeta::new_readonly(fund.fund_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(wd_custody_token_account, false),
            AccountMeta::new(wd_custody_metadata, false),
            AccountMeta::new(trading_custody_token_account, false),
            AccountMeta::new(trading_custody_metadata, false),
            AccountMeta::new_readonly(token_ref, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for initiating liquidation of the Fund
    pub fn new_instruction_start_liquidation_fund(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;
        let fund_token = self.get_token_by_ref(&fund.fund_token_ref)?;
        let user_info_account = self.get_fund_user_info_account(wallet_address, fund_name)?;
        let user_fund_token_account =
            self.get_associated_token_address(wallet_address, fund_token.name.as_str())?;

        // fill in accounts and instruction data
        let data = FundInstruction::StartLiquidation.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(fund_token.mint, false),
            AccountMeta::new_readonly(user_info_account, false),
            AccountMeta::new_readonly(user_fund_token_account, false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for stopping liquidation of the Fund
    pub fn new_instruction_stop_liquidation_fund(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // fill in accounts and instruction data
        let data = FundInstruction::StopLiquidation.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for fees withdrawal from the Fund
    pub fn new_instruction_withdraw_fees_fund(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        custody_type: FundCustodyType,
        ui_amount: f64,
        receiver: &Pubkey,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // get custodies
        let custody_fees_token_account =
            self.get_fund_custody_fees_token_account(fund_name, token_name, custody_type)?;

        // fill in accounts and instruction data
        let token = self.get_token(token_name)?;
        let data = FundInstruction::WithdrawFees {
            amount: self.ui_amount_to_tokens_with_decimals(ui_amount, token.decimals)?,
        }
        .to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(self.get_fund_active_multisig_account(fund_name)?, false),
            AccountMeta::new(self.get_fund_multisig_account(fund_name)?, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new(custody_fees_token_account, false),
            AccountMeta::new(*receiver, false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for updating Fund assets based on custody holdings
    pub fn new_instruction_update_fund_assets_with_custody(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        custody_id: u32,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // get custodies
        let custodies = self.get_fund_custodies(fund_name)?;
        let custody = custodies
            .iter()
            .find(|&c| c.custody_id == custody_id)
            .ok_or_else(|| {
                FarmClientError::RecordNotFound(format!("Custody with ID {}", custody_id))
            })?;
        let token = self.get_token_by_ref(&custody.token_ref)?;
        let custody_metadata =
            self.get_fund_custody_account(fund_name, token.name.as_str(), custody.custody_type)?;
        let custodies_assets_account =
            self.get_fund_assets_account(fund_name, FundAssetType::Custody)?;
        let vaults_assets_account =
            self.get_fund_assets_account(fund_name, FundAssetType::Vault)?;
        let custody_token_account = self.get_fund_custody_token_account(
            fund_name,
            token.name.as_str(),
            custody.custody_type,
        )?;
        let (_, oracle_account) = self.get_oracle(&token.name)?;

        // fill in accounts and instruction data
        let accounts = vec![
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new(custodies_assets_account, false),
            AccountMeta::new_readonly(vaults_assets_account, false),
            AccountMeta::new(custody_token_account, false),
            AccountMeta::new(custody_metadata, false),
            AccountMeta::new_readonly(custody.token_ref, false),
            AccountMeta::new_readonly(oracle_account.unwrap_or_else(zero::id), false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data: FundInstruction::UpdateAssetsWithCustody.to_vec()?,
            accounts,
        })
    }

    /// Creates a new Instruction for updating Fund assets with Vault holdings
    pub fn new_instruction_update_fund_assets_with_vault(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        vault_id: u32,
    ) -> Result<Instruction, FarmClientError> {
        // get fund info
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        // get vaults
        let vaults = self.get_fund_vaults(fund_name)?;
        let vault = vaults
            .iter()
            .find(|&c| c.vault_id == vault_id)
            .ok_or_else(|| {
                FarmClientError::RecordNotFound(format!("Fund Vault with ID {}", vault_id))
            })?;
        if vault.vault_type == FundVaultType::Farm {
            return Err(FarmClientError::ValueError(
                "Nothing to do: Farms are not processed to avoid double counting".to_string(),
            ));
        }
        let vault_name = match vault.vault_type {
            FundVaultType::Vault => self.get_vault_by_ref(&vault.vault_ref)?.name,
            FundVaultType::Pool => self.get_pool_by_ref(&vault.vault_ref)?.name,
            FundVaultType::Farm => unreachable!(),
        };
        let token_names = match vault.vault_type {
            FundVaultType::Vault => self.get_vault_token_names(&vault_name)?,
            FundVaultType::Pool => self.get_pool_token_names(&vault_name)?,
            FundVaultType::Farm => unreachable!(),
        };
        let target_vault_metadata = match vault.vault_type {
            FundVaultType::Vault => self.get_vault_ref(&vault_name)?,
            FundVaultType::Pool => self.get_pool_ref(&vault_name)?,
            FundVaultType::Farm => unreachable!(),
        };
        let underlying_pool_ref = if vault.vault_type == FundVaultType::Vault {
            match self.get_vault(&vault_name)?.strategy {
                VaultStrategy::StakeLpCompoundRewards { pool_ref, .. } => pool_ref,
                _ => unreachable!(),
            }
        } else {
            target_vault_metadata
        };
        let underlying_pool = self.get_pool_by_ref(&underlying_pool_ref)?;
        let underlying_lp_token = self.get_token_by_ref(
            &underlying_pool
                .lp_token_ref
                .ok_or(ProgramError::UninitializedAccount)?,
        )?;
        let (amm_id, amm_open_orders) = match underlying_pool.route {
            PoolRoute::Raydium {
                amm_id,
                amm_open_orders,
                ..
            } => (amm_id, amm_open_orders),
            PoolRoute::Orca { amm_id, .. } => (amm_id, zero::id()),
            _ => {
                return Err(FarmClientError::ValueError(
                    "Unsupported pool route".to_string(),
                ));
            }
        };
        let vault_metadata =
            self.get_fund_vault_account(fund_name, vault_name.as_str(), vault.vault_type)?;
        let custodies_assets_account =
            self.get_fund_assets_account(fund_name, FundAssetType::Custody)?;
        let vaults_assets_account =
            self.get_fund_assets_account(fund_name, FundAssetType::Vault)?;
        let (_, oracle_account_token_a) = if token_names.0.is_empty() {
            (OracleType::Unsupported, None)
        } else {
            self.get_oracle(&token_names.0)?
        };
        let (_, oracle_account_token_b) = if token_names.1.is_empty() {
            (OracleType::Unsupported, None)
        } else {
            self.get_oracle(&token_names.1)?
        };

        // fill in accounts and instruction data
        let accounts = vec![
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            AccountMeta::new_readonly(custodies_assets_account, false),
            AccountMeta::new(vaults_assets_account, false),
            AccountMeta::new(vault_metadata, false),
            AccountMeta::new_readonly(vault.vault_ref, false),
            AccountMeta::new_readonly(underlying_pool_ref, false),
            AccountMeta::new_readonly(
                underlying_pool
                    .token_a_ref
                    .ok_or(ProgramError::UninitializedAccount)?,
                false,
            ),
            AccountMeta::new_readonly(
                underlying_pool
                    .token_b_ref
                    .ok_or(ProgramError::UninitializedAccount)?,
                false,
            ),
            AccountMeta::new_readonly(underlying_lp_token.mint, false),
            AccountMeta::new_readonly(
                underlying_pool
                    .token_a_account
                    .ok_or(ProgramError::UninitializedAccount)?,
                false,
            ),
            AccountMeta::new_readonly(
                underlying_pool
                    .token_b_account
                    .ok_or(ProgramError::UninitializedAccount)?,
                false,
            ),
            AccountMeta::new_readonly(amm_id, false),
            AccountMeta::new_readonly(amm_open_orders, false),
            AccountMeta::new_readonly(oracle_account_token_a.unwrap_or_else(zero::id), false),
            AccountMeta::new_readonly(oracle_account_token_b.unwrap_or_else(zero::id), false),
            AccountMeta::new_readonly(sysvar::instructions::id(), false),
        ];

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data: FundInstruction::UpdateAssetsWithVault.to_vec()?,
            accounts,
        })
    }

    /// Creates a new complete set of Instructions for requesting a new deposit to the Fund
    pub fn all_instructions_request_deposit_fund(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Vec<Instruction>, FarmClientError> {
        let mut inst = Vec::<Instruction>::new();
        let _ =
            self.check_fund_accounts(wallet_address, fund_name, token_name, ui_amount, &mut inst)?;

        // create and send the instruction
        inst.push(self.new_instruction_request_deposit_fund(
            wallet_address,
            fund_name,
            token_name,
            ui_amount,
        )?);

        Ok(inst)
    }

    /// Creates a new complete set of Instructions for requesting a new withdrawal from the Fund
    pub fn all_instructions_request_withdrawal_fund(
        &self,
        wallet_address: &Pubkey,
        fund_name: &str,
        token_name: &str,
        ui_amount: f64,
    ) -> Result<Vec<Instruction>, FarmClientError> {
        let mut inst = Vec::<Instruction>::new();
        let fund = self.get_fund(fund_name)?;
        let fund_token = Some(self.get_token_by_ref(&fund.fund_token_ref)?);
        let asset_token = Some(self.get_token(token_name)?);
        let _ = self.check_token_account(wallet_address, &fund_token, ui_amount, &mut inst)?;
        let _ = self.check_token_account(wallet_address, &asset_token, 0.0, &mut inst)?;

        if self
            .get_fund_user_requests(wallet_address, fund_name, token_name)
            .is_err()
        {
            inst.push(self.new_instruction_user_init_fund(
                wallet_address,
                fund_name,
                token_name,
            )?);
        }

        // create and send the instruction
        inst.push(self.new_instruction_request_withdrawal_fund(
            wallet_address,
            fund_name,
            token_name,
            ui_amount,
        )?);

        Ok(inst)
    }
}
