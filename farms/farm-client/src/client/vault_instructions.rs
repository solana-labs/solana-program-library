//! Solana Farm Client Vault Instructions

use {
    crate::error::FarmClientError,
    solana_farm_sdk::{
        instruction::vault::VaultInstruction, pool::PoolRoute, token::TokenSelector,
        vault::VaultStrategy,
    },
    solana_sdk::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};

use super::FarmClient;

impl FarmClient {
    /// Creates a new Instruction for initializing a new User for the Vault
    pub fn new_instruction_user_init_vault(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;

        // fill in accounts and instruction data
        let (accounts, data) = match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards { pool_id_ref, .. } => {
                let pool = self.get_pool_by_ref(&pool_id_ref)?;
                match pool.route {
                    PoolRoute::Raydium { .. } => {
                        self.get_stc_user_init_accounts_raydium(wallet_address, vault_name)
                    }
                    PoolRoute::Saber { .. } => {
                        self.get_stc_user_init_accounts_saber(wallet_address, vault_name)
                    }
                    _ => unreachable!(),
                }
            }
            _ => {
                unreachable!()
            }
        }?;

        Ok(Instruction {
            program_id: vault.vault_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for adding liquidity to the Vault
    pub fn new_instruction_add_liquidity_vault(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
        max_token_a_ui_amount: f64,
        max_token_b_ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;

        // fill in accounts and instruction data
        let (accounts, data) = match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards { pool_id_ref, .. } => {
                let pool = self.get_pool_by_ref(&pool_id_ref)?;
                match pool.route {
                    PoolRoute::Raydium { .. } => self.get_stc_add_liquidity_accounts_raydium(
                        wallet_address,
                        vault_name,
                        max_token_a_ui_amount,
                        max_token_b_ui_amount,
                    ),
                    PoolRoute::Saber { .. } => self.get_stc_add_liquidity_accounts_saber(
                        wallet_address,
                        vault_name,
                        max_token_a_ui_amount,
                        max_token_b_ui_amount,
                    ),
                    _ => unreachable!(),
                }
            }
            _ => {
                unreachable!()
            }
        }?;

        Ok(Instruction {
            program_id: vault.vault_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for locking liquidity in the Vault
    pub fn new_instruction_lock_liquidity_vault(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;

        // fill in accounts and instruction data
        let (accounts, data) = match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards { pool_id_ref, .. } => {
                let pool = self.get_pool_by_ref(&pool_id_ref)?;
                match pool.route {
                    PoolRoute::Raydium { .. } => Err(FarmClientError::ValueError(format!(
                        "LockLiquidity is not supported by Vault {}",
                        vault_name
                    ))),
                    PoolRoute::Saber { .. } => self.get_stc_lock_liquidity_accounts_saber(
                        wallet_address,
                        vault_name,
                        ui_amount,
                    ),
                    _ => unreachable!(),
                }
            }
            _ => {
                unreachable!()
            }
        }?;

        Ok(Instruction {
            program_id: vault.vault_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for unlocking liquidity in the Vault
    pub fn new_instruction_unlock_liquidity_vault(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;

        // fill in accounts and instruction data
        let (accounts, data) = match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards { pool_id_ref, .. } => {
                let pool = self.get_pool_by_ref(&pool_id_ref)?;
                match pool.route {
                    PoolRoute::Raydium { .. } => self.get_stc_unlock_liquidity_accounts_raydium(
                        wallet_address,
                        vault_name,
                        ui_amount,
                    ),
                    PoolRoute::Saber { .. } => Err(FarmClientError::ValueError(format!(
                        "LockLiquidity is not supported by Vault {}",
                        vault_name
                    ))),
                    _ => unreachable!(),
                }
            }
            _ => {
                unreachable!()
            }
        }?;

        Ok(Instruction {
            program_id: vault.vault_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for removing liquidity from the Vault
    pub fn new_instruction_remove_liquidity_vault(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;

        // fill in accounts and instruction data
        let (accounts, data) = match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards { pool_id_ref, .. } => {
                let pool = self.get_pool_by_ref(&pool_id_ref)?;
                match pool.route {
                    PoolRoute::Raydium { .. } => self.get_stc_remove_liquidity_accounts_raydium(
                        wallet_address,
                        vault_name,
                        ui_amount,
                    ),
                    PoolRoute::Saber { .. } => self.get_stc_remove_liquidity_accounts_saber(
                        wallet_address,
                        vault_name,
                        ui_amount,
                    ),
                    _ => unreachable!(),
                }
            }
            _ => {
                unreachable!()
            }
        }?;

        Ok(Instruction {
            program_id: vault.vault_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Vault Init Instruction
    pub fn new_instruction_init_vault(
        &self,
        admin_address: &Pubkey,
        vault_name: &str,
        step: u64,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;

        // fill in accounts and instruction data
        let (accounts, data) = match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards { pool_id_ref, .. } => {
                let pool = self.get_pool_by_ref(&pool_id_ref)?;
                match pool.route {
                    PoolRoute::Raydium { .. } => {
                        self.get_stc_init_accounts_raydium(admin_address, vault_name, step)
                    }
                    PoolRoute::Saber { .. } => {
                        self.get_stc_init_accounts_saber(admin_address, vault_name, step)
                    }
                    _ => unreachable!(),
                }
            }
            _ => {
                unreachable!()
            }
        }?;

        Ok(Instruction {
            program_id: vault.vault_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Vault Shutdown Instruction
    pub fn new_instruction_shutdown_vault(
        &self,
        admin_address: &Pubkey,
        vault_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;

        // fill in accounts and instruction data
        let (accounts, data) = match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards { pool_id_ref, .. } => {
                let pool = self.get_pool_by_ref(&pool_id_ref)?;
                match pool.route {
                    PoolRoute::Raydium { .. } => {
                        self.get_stc_shutdown_accounts_raydium(admin_address, vault_name)
                    }
                    PoolRoute::Saber { .. } => {
                        self.get_stc_shutdown_accounts_saber(admin_address, vault_name)
                    }
                    _ => unreachable!(),
                }
            }
            _ => {
                unreachable!()
            }
        }?;

        Ok(Instruction {
            program_id: vault.vault_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new instruction for withdrawal collected fees from the Vault
    pub fn new_instruction_withdraw_fees_vault(
        &self,
        admin_address: &Pubkey,
        vault_name: &str,
        fee_token: TokenSelector,
        ui_amount: f64,
        receiver: &Pubkey,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;

        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: vault.vault_program_id,
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new_readonly(vault_ref, false),
                AccountMeta::new(vault.info_account, false),
                AccountMeta::new(vault.vault_authority, false),
                AccountMeta::new_readonly(spl_token::id(), false),
                if fee_token == TokenSelector::TokenA {
                    AccountMeta::new(
                        vault
                            .fees_account_a
                            .ok_or(ProgramError::UninitializedAccount)?,
                        false,
                    )
                } else {
                    AccountMeta::new(
                        vault
                            .fees_account_b
                            .ok_or(ProgramError::UninitializedAccount)?,
                        false,
                    )
                },
                AccountMeta::new(*receiver, false),
            ],
        };

        let fee_decimals =
            if let VaultStrategy::StakeLpCompoundRewards { farm_id_ref, .. } = vault.strategy {
                let farm = self.get_farm_by_ref(&farm_id_ref)?;
                if fee_token == TokenSelector::TokenA {
                    let token_a_reward = self
                        .get_token_by_ref_from_cache(&farm.reward_token_a_ref)?
                        .unwrap();
                    token_a_reward.decimals
                } else {
                    let token_b_reward = self
                        .get_token_by_ref_from_cache(&farm.reward_token_b_ref)?
                        .unwrap();
                    token_b_reward.decimals
                }
            } else {
                unreachable!();
            };

        inst.data = VaultInstruction::WithdrawFees {
            amount: self.ui_amount_to_tokens_with_decimals(ui_amount, fee_decimals),
        }
        .to_vec()?;

        Ok(inst)
    }

    /// Creates a new Vault Crank Instruction
    pub fn new_instruction_crank_vault(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
        step: u64,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;

        // fill in accounts and instruction data
        let (accounts, data) = match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards { pool_id_ref, .. } => {
                let pool = self.get_pool_by_ref(&pool_id_ref)?;
                match pool.route {
                    PoolRoute::Raydium { .. } => {
                        self.get_stc_crank_accounts_raydium(wallet_address, vault_name, step)
                    }
                    PoolRoute::Saber { .. } => {
                        self.get_stc_crank_accounts_saber(wallet_address, vault_name, step)
                    }
                    _ => unreachable!(),
                }
            }
            _ => {
                unreachable!()
            }
        }?;

        Ok(Instruction {
            program_id: vault.vault_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for updating the Vault's min crank interval
    pub fn new_instruction_set_min_crank_interval_vault(
        &self,
        admin_address: &Pubkey,
        vault_name: &str,
        min_crank_interval: u32,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;

        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: vault.vault_program_id,
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new_readonly(vault_ref, false),
                AccountMeta::new(vault.info_account, false),
            ],
        };

        inst.data = VaultInstruction::SetMinCrankInterval { min_crank_interval }.to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for updating the Vault's fee
    pub fn new_instruction_set_fee_vault(
        &self,
        admin_address: &Pubkey,
        vault_name: &str,
        fee_percent: f32,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;

        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: vault.vault_program_id,
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new_readonly(vault_ref, false),
                AccountMeta::new(vault.info_account, false),
            ],
        };

        inst.data = VaultInstruction::SetFee {
            fee: fee_percent * 0.01,
        }
        .to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for updating the Vault's external fee
    pub fn new_instruction_set_external_fee_vault(
        &self,
        admin_address: &Pubkey,
        vault_name: &str,
        external_fee_percent: f32,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;

        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: vault.vault_program_id,
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new_readonly(vault_ref, false),
                AccountMeta::new(vault.info_account, false),
            ],
        };

        inst.data = VaultInstruction::SetExternalFee {
            external_fee: external_fee_percent * 0.01,
        }
        .to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for disabling deposits to the Vault
    pub fn new_instruction_disable_deposit_vault(
        &self,
        admin_address: &Pubkey,
        vault_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;

        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: vault.vault_program_id,
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new_readonly(vault_ref, false),
                AccountMeta::new(vault.info_account, false),
            ],
        };

        inst.data = VaultInstruction::DisableDeposit.to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for enabling deposits to the Vault
    pub fn new_instruction_enable_deposit_vault(
        &self,
        admin_address: &Pubkey,
        vault_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;

        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: vault.vault_program_id,
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new_readonly(vault_ref, false),
                AccountMeta::new(vault.info_account, false),
            ],
        };

        inst.data = VaultInstruction::EnableDeposit.to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for disabling withdrawals from the Vault
    pub fn new_instruction_disable_withdrawal_vault(
        &self,
        admin_address: &Pubkey,
        vault_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;

        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: vault.vault_program_id,
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new_readonly(vault_ref, false),
                AccountMeta::new(vault.info_account, false),
            ],
        };

        inst.data = VaultInstruction::DisableWithdrawal.to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for enabling withdrawals from the Vault
    pub fn new_instruction_enable_withdrawal_vault(
        &self,
        admin_address: &Pubkey,
        vault_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;

        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: vault.vault_program_id,
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new_readonly(vault_ref, false),
                AccountMeta::new(vault.info_account, false),
            ],
        };

        inst.data = VaultInstruction::EnableWithdrawal.to_vec()?;

        Ok(inst)
    }
}
