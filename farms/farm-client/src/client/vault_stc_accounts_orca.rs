//! Solana Farm Client Vault Instructions

use {
    crate::error::FarmClientError,
    solana_farm_sdk::{
        farm::FarmRoute, id::zero, instruction::vault::VaultInstruction, pool::PoolRoute,
        vault::VaultStrategy,
    },
    solana_sdk::{
        instruction::AccountMeta, program_error::ProgramError, pubkey::Pubkey, system_program,
        sysvar,
    },
    std::vec::Vec,
};

use super::FarmClient;

impl FarmClient {
    /// Returns accounts and data for initializing a new User for the Vault
    pub fn get_stc_user_init_accounts_orca(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
    ) -> Result<(Vec<AccountMeta>, Vec<u8>), FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;
        // fill in accounts and instruction data
        let data = VaultInstruction::UserInit.to_vec()?;
        let accounts = vec![
            AccountMeta::new(*wallet_address, true),
            AccountMeta::new_readonly(vault_ref, false),
            AccountMeta::new(vault.info_account, false),
            AccountMeta::new_readonly(*wallet_address, false),
            AccountMeta::new(
                self.get_vault_user_info_account(wallet_address, vault_name)?,
                false,
            ),
            AccountMeta::new_readonly(system_program::id(), false),
        ];
        Ok((accounts, data))
    }

    /// Returns accounts and data for adding liquidity to the Vault
    pub fn get_stc_add_liquidity_accounts_orca(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
        max_token_a_ui_amount: f64,
        max_token_b_ui_amount: f64,
    ) -> Result<(Vec<AccountMeta>, Vec<u8>), FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;

        // fill in accounts and instruction data
        let mut accounts = vec![AccountMeta::new_readonly(*wallet_address, true)];

        // general accounts
        accounts.push(AccountMeta::new_readonly(vault_ref, false));
        accounts.push(AccountMeta::new(vault.info_account, false));
        accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
        accounts.push(AccountMeta::new(
            self.get_vault_user_info_account(wallet_address, vault_name)?,
            false,
        ));

        // strategy related accounts
        let data = match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards {
                pool_ref,
                farm_ref,
                lp_token_custody,
                ..
            } => {
                let pool = self.get_pool_by_ref(&pool_ref)?;
                let farm = self.get_farm_by_ref(&farm_ref)?;

                // get tokens info
                let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
                let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;
                let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;
                let farm_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;
                assert_eq!(farm_token, lp_token);

                // get user accounts info
                let user_token_a_account = self.get_token_account(wallet_address, &token_a);
                let user_token_b_account = self.get_token_account(wallet_address, &token_b);
                let user_lp_token_account = self.get_token_account(wallet_address, &lp_token);

                // fill in pool related accounts
                match pool.route {
                    PoolRoute::Orca {
                        amm_id,
                        amm_authority,
                        ..
                    } => {
                        accounts.push(AccountMeta::new(
                            user_token_a_account.ok_or(ProgramError::UninitializedAccount)?,
                            false,
                        ));
                        accounts.push(AccountMeta::new(
                            user_token_b_account.ok_or(ProgramError::UninitializedAccount)?,
                            false,
                        ));
                        accounts.push(AccountMeta::new(
                            user_lp_token_account.ok_or(ProgramError::UninitializedAccount)?,
                            false,
                        ));
                        accounts.push(AccountMeta::new(lp_token_custody, false));
                        accounts.push(AccountMeta::new_readonly(pool.pool_program_id, false));
                        accounts.push(AccountMeta::new(
                            pool.token_a_account
                                .ok_or(ProgramError::UninitializedAccount)?,
                            false,
                        ));
                        accounts.push(AccountMeta::new(
                            pool.token_b_account
                                .ok_or(ProgramError::UninitializedAccount)?,
                            false,
                        ));
                        accounts.push(AccountMeta::new(
                            lp_token.ok_or(ProgramError::UninitializedAccount)?.mint,
                            false,
                        ));
                        accounts.push(AccountMeta::new(amm_id, false));
                        accounts.push(AccountMeta::new_readonly(amm_authority, false));
                    }
                    _ => {
                        unreachable!();
                    }
                }

                VaultInstruction::AddLiquidity {
                    max_token_a_amount: self
                        .to_token_amount_option(max_token_a_ui_amount, &token_a)?,
                    max_token_b_amount: self
                        .to_token_amount_option(max_token_b_ui_amount, &token_b)?,
                }
                .to_vec()?
            }
            VaultStrategy::DynamicHedge { .. } => {
                unreachable!();
            }
        };
        Ok((accounts, data))
    }

    /// Returns accounts and data for locking liquidity in the Vault
    pub fn get_stc_lock_liquidity_accounts_orca(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<(Vec<AccountMeta>, Vec<u8>), FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;
        let vault_token = self.get_token_by_ref_from_cache(&Some(vault.vault_token_ref))?;

        // fill in accounts and instruction data
        let mut accounts = vec![AccountMeta::new_readonly(*wallet_address, true)];

        // general accounts
        accounts.push(AccountMeta::new_readonly(vault_ref, false));
        accounts.push(AccountMeta::new(vault.info_account, false));
        accounts.push(AccountMeta::new_readonly(vault.vault_authority, false));
        accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
        accounts.push(AccountMeta::new(vault_token.unwrap().mint, false));
        accounts.push(AccountMeta::new(
            self.get_vault_user_info_account(wallet_address, vault_name)?,
            false,
        ));

        // strategy related accounts
        let data = match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards {
                pool_ref,
                farm_ref,
                lp_token_custody,
                token_a_reward_custody,
                vault_stake_info,
                vault_stake_custody,
                ..
            } => {
                let pool = self.get_pool_by_ref(&pool_ref)?;
                let farm = self.get_farm_by_ref(&farm_ref)?;

                // get tokens info
                let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;
                let farm_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;
                assert_eq!(farm_token, lp_token);

                // get user accounts info
                let user_vt_token_account = self.get_token_account(wallet_address, &vault_token);

                accounts.push(AccountMeta::new(
                    user_vt_token_account.ok_or(ProgramError::UninitializedAccount)?,
                    false,
                ));
                accounts.push(AccountMeta::new(token_a_reward_custody, false));
                accounts.push(AccountMeta::new(lp_token_custody, false));

                // fill in farm related accounts
                match farm.route {
                    FarmRoute::Orca {
                        farm_id,
                        farm_authority,
                        farm_token_ref,
                        base_token_vault,
                        reward_token_vault,
                    } => {
                        let farm_lp_token =
                            self.get_token_by_ref_from_cache(&Some(farm_token_ref))?;

                        accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
                        accounts.push(AccountMeta::new(vault_stake_info, false));
                        accounts.push(AccountMeta::new(
                            vault_stake_custody.ok_or(ProgramError::UninitializedAccount)?,
                            false,
                        ));
                        accounts.push(AccountMeta::new(farm_id, false));
                        accounts.push(AccountMeta::new_readonly(farm_authority, false));
                        accounts.push(AccountMeta::new(
                            farm_lp_token
                                .ok_or(ProgramError::UninitializedAccount)?
                                .mint,
                            false,
                        ));
                        accounts.push(AccountMeta::new(base_token_vault, false));
                        accounts.push(AccountMeta::new(reward_token_vault, false));
                    }
                    _ => {
                        unreachable!();
                    }
                }

                VaultInstruction::LockLiquidity {
                    amount: self.to_token_amount_option(ui_amount, &lp_token)?,
                }
                .to_vec()?
            }
            VaultStrategy::DynamicHedge { .. } => {
                unreachable!();
            }
        };
        Ok((accounts, data))
    }

    /// Returns accounts and data for unlocking liquidity in the Vault
    pub fn get_stc_unlock_liquidity_accounts_orca(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<(Vec<AccountMeta>, Vec<u8>), FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;
        let vault_token = self.get_token_by_ref_from_cache(&Some(vault.vault_token_ref))?;

        // fill in accounts and instruction data
        let data = VaultInstruction::UnlockLiquidity {
            amount: self.to_token_amount_option(ui_amount, &vault_token)?,
        }
        .to_vec()?;
        let mut accounts = vec![AccountMeta::new_readonly(*wallet_address, true)];

        // general accounts
        accounts.push(AccountMeta::new_readonly(vault_ref, false));
        accounts.push(AccountMeta::new(vault.info_account, false));
        accounts.push(AccountMeta::new_readonly(vault.vault_authority, false));
        accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
        accounts.push(AccountMeta::new(vault_token.unwrap().mint, false));
        accounts.push(AccountMeta::new(
            self.get_vault_user_info_account(wallet_address, vault_name)?,
            false,
        ));

        // strategy related accounts
        match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards {
                pool_ref,
                farm_ref,
                lp_token_custody,
                token_a_reward_custody,
                vault_stake_info,
                vault_stake_custody,
                ..
            } => {
                let pool = self.get_pool_by_ref(&pool_ref)?;
                let farm = self.get_farm_by_ref(&farm_ref)?;

                // get tokens info
                let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;
                let farm_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;
                assert_eq!(farm_token, lp_token);

                // get user accounts info
                let user_vt_token_account = self.get_token_account(wallet_address, &vault_token);

                accounts.push(AccountMeta::new(
                    user_vt_token_account.ok_or(ProgramError::UninitializedAccount)?,
                    false,
                ));
                accounts.push(AccountMeta::new(token_a_reward_custody, false));
                accounts.push(AccountMeta::new(lp_token_custody, false));

                // fill in farm related accounts
                match farm.route {
                    FarmRoute::Orca {
                        farm_id,
                        farm_authority,
                        farm_token_ref,
                        base_token_vault,
                        reward_token_vault,
                    } => {
                        let farm_lp_token =
                            self.get_token_by_ref_from_cache(&Some(farm_token_ref))?;

                        accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
                        accounts.push(AccountMeta::new(vault_stake_info, false));
                        accounts.push(AccountMeta::new(
                            vault_stake_custody.ok_or(ProgramError::UninitializedAccount)?,
                            false,
                        ));
                        accounts.push(AccountMeta::new(farm_id, false));
                        accounts.push(AccountMeta::new_readonly(farm_authority, false));
                        accounts.push(AccountMeta::new(
                            farm_lp_token
                                .ok_or(ProgramError::UninitializedAccount)?
                                .mint,
                            false,
                        ));
                        accounts.push(AccountMeta::new(base_token_vault, false));
                        accounts.push(AccountMeta::new(reward_token_vault, false));
                    }
                    _ => {
                        unreachable!();
                    }
                }
            }
            VaultStrategy::DynamicHedge { .. } => {
                unreachable!();
            }
        }
        Ok((accounts, data))
    }

    /// Returns accounts and data for removing liquidity from the Vault
    pub fn get_stc_remove_liquidity_accounts_orca(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<(Vec<AccountMeta>, Vec<u8>), FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;
        // fill in accounts and instruction data
        let mut accounts = vec![AccountMeta::new_readonly(*wallet_address, true)];

        // general accounts
        accounts.push(AccountMeta::new_readonly(vault_ref, false));
        accounts.push(AccountMeta::new(vault.info_account, false));
        accounts.push(AccountMeta::new_readonly(vault.vault_authority, false));
        accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
        accounts.push(AccountMeta::new(
            self.get_vault_user_info_account(wallet_address, vault_name)?,
            false,
        ));

        // strategy related accounts
        let data = match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards {
                pool_ref,
                farm_ref,
                lp_token_custody,
                ..
            } => {
                let pool = self.get_pool_by_ref(&pool_ref)?;
                let farm = self.get_farm_by_ref(&farm_ref)?;

                // get tokens info
                let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
                let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;
                let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;
                let farm_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;
                assert_eq!(farm_token, lp_token);

                // get user accounts info
                let user_token_a_account = self.get_token_account(wallet_address, &token_a);
                let user_token_b_account = self.get_token_account(wallet_address, &token_b);

                // fill in pool related accounts
                match pool.route {
                    PoolRoute::Orca {
                        amm_id,
                        amm_authority,
                        fees_account,
                    } => {
                        accounts.push(AccountMeta::new(
                            user_token_a_account.ok_or(ProgramError::UninitializedAccount)?,
                            false,
                        ));
                        accounts.push(AccountMeta::new(
                            user_token_b_account.ok_or(ProgramError::UninitializedAccount)?,
                            false,
                        ));
                        accounts.push(AccountMeta::new(lp_token_custody, false));
                        accounts.push(AccountMeta::new_readonly(pool.pool_program_id, false));
                        accounts.push(AccountMeta::new(
                            pool.token_a_account
                                .ok_or(ProgramError::UninitializedAccount)?,
                            false,
                        ));
                        accounts.push(AccountMeta::new(
                            pool.token_b_account
                                .ok_or(ProgramError::UninitializedAccount)?,
                            false,
                        ));
                        accounts.push(AccountMeta::new(
                            lp_token.ok_or(ProgramError::UninitializedAccount)?.mint,
                            false,
                        ));
                        accounts.push(AccountMeta::new(amm_id, false));
                        accounts.push(AccountMeta::new_readonly(amm_authority, false));
                        accounts.push(AccountMeta::new(fees_account, false));
                    }
                    _ => {
                        unreachable!();
                    }
                }

                VaultInstruction::RemoveLiquidity {
                    amount: self.to_token_amount_option(ui_amount, &lp_token)?,
                }
                .to_vec()?
            }
            VaultStrategy::DynamicHedge { .. } => {
                unreachable!();
            }
        };
        Ok((accounts, data))
    }

    /// Returns accounts and data for a Vault Init Instruction
    pub fn get_stc_init_accounts_orca(
        &self,
        admin_address: &Pubkey,
        vault_name: &str,
        step: u64,
    ) -> Result<(Vec<AccountMeta>, Vec<u8>), FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;
        let vault_token = self
            .get_token_by_ref_from_cache(&Some(vault.vault_token_ref))?
            .unwrap();

        // fill in accounts and instruction data
        let data = VaultInstruction::Init { step }.to_vec()?;
        let mut accounts = vec![AccountMeta::new_readonly(*admin_address, true)];

        // general accounts
        accounts.push(AccountMeta::new_readonly(vault_ref, false));
        accounts.push(AccountMeta::new(vault.info_account, false));
        accounts.push(AccountMeta::new(
            self.get_vault_active_multisig_account(vault_name)?,
            false,
        ));
        accounts.push(AccountMeta::new(vault.vault_authority, false));
        accounts.push(AccountMeta::new_readonly(vault.vault_program_id, false));
        accounts.push(AccountMeta::new_readonly(system_program::id(), false));
        accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
        accounts.push(AccountMeta::new_readonly(sysvar::rent::id(), false));

        match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards {
                pool_ref,
                farm_id,
                farm_ref,
                lp_token_custody,
                token_a_custody,
                token_b_custody,
                token_a_reward_custody,
                vault_stake_info,
                vault_stake_custody,
                ..
            } => {
                // get pools
                let pool = self.get_pool_by_ref(&pool_ref)?;
                let farm = self.get_farm_by_ref(&farm_ref)?;
                // get tokens info
                let token_a = self
                    .get_token_by_ref_from_cache(&pool.token_a_ref)?
                    .unwrap();
                let token_b = self
                    .get_token_by_ref_from_cache(&pool.token_b_ref)?
                    .unwrap();
                let lp_token = self
                    .get_token_by_ref_from_cache(&pool.lp_token_ref)?
                    .unwrap();
                let farm_token_ref = match farm.route {
                    FarmRoute::Orca { farm_token_ref, .. } => farm_token_ref,
                    _ => unreachable!(),
                };
                let farm_lp_token = self.get_token_by_ref_from_cache(&Some(farm_token_ref))?;
                let token_a_reward = self
                    .get_token_by_ref_from_cache(&farm.first_reward_token_ref)?
                    .unwrap();

                accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
                accounts.push(AccountMeta::new(vault_token.mint, false));
                accounts.push(AccountMeta::new_readonly(vault.vault_token_ref, false));
                accounts.push(AccountMeta::new(vault_stake_info, false));
                accounts.push(AccountMeta::new(
                    vault_stake_custody.ok_or(ProgramError::UninitializedAccount)?,
                    false,
                ));
                accounts.push(AccountMeta::new(
                    vault.fees_account_a.or_else(|| Some(zero::id())).unwrap(),
                    false,
                ));
                accounts.push(AccountMeta::new(token_a_custody, false));
                accounts.push(AccountMeta::new(
                    token_b_custody.or_else(|| Some(zero::id())).unwrap(),
                    false,
                ));
                accounts.push(AccountMeta::new(lp_token_custody, false));
                accounts.push(AccountMeta::new(token_a.mint, false));
                accounts.push(AccountMeta::new(token_b.mint, false));
                accounts.push(AccountMeta::new(lp_token.mint, false));
                accounts.push(AccountMeta::new(
                    farm_lp_token
                        .ok_or(ProgramError::UninitializedAccount)?
                        .mint,
                    false,
                ));
                accounts.push(AccountMeta::new(token_a_reward_custody, false));
                accounts.push(AccountMeta::new(token_a_reward.mint, false));
                accounts.push(AccountMeta::new_readonly(farm_id, false));
            }
            VaultStrategy::DynamicHedge { .. } => {
                unreachable!();
            }
        }

        Ok((accounts, data))
    }

    /// Returns accounts and data for a Vault Shutdown Instruction
    pub fn get_stc_shutdown_accounts_orca(
        &self,
        admin_address: &Pubkey,
        vault_name: &str,
    ) -> Result<(Vec<AccountMeta>, Vec<u8>), FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;

        // fill in accounts and instruction data
        let data = VaultInstruction::Shutdown.to_vec()?;
        let accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(vault_ref, false),
            AccountMeta::new(vault.info_account, false),
            AccountMeta::new(self.get_vault_active_multisig_account(vault_name)?, false),
        ];

        Ok((accounts, data))
    }

    /// Returns accounts and data for a Vault Crank Instruction
    pub fn get_stc_crank_accounts_orca(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
        step: u64,
    ) -> Result<(Vec<AccountMeta>, Vec<u8>), FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;

        // fill in accounts and instruction data
        let data = VaultInstruction::Crank { step }.to_vec()?;
        let mut accounts = vec![AccountMeta::new_readonly(*wallet_address, true)];

        // general accounts
        accounts.push(AccountMeta::new_readonly(vault_ref, false));
        accounts.push(AccountMeta::new(vault.info_account, false));
        accounts.push(AccountMeta::new_readonly(vault.vault_authority, false));
        accounts.push(AccountMeta::new_readonly(spl_token::id(), false));

        // strategy related accounts
        match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards {
                pool_ref,
                farm_ref,
                lp_token_custody,
                token_a_custody,
                token_b_custody,
                token_a_reward_custody,
                vault_stake_info,
                vault_stake_custody,
                reward_exchange_pool_ref,
                ..
            } => {
                let pool = self.get_pool_by_ref(&pool_ref)?;
                let farm = self.get_farm_by_ref(&farm_ref)?;

                // get tokens info
                let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;
                let farm_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;
                assert_eq!(farm_token, lp_token);

                accounts.push(AccountMeta::new(token_a_reward_custody, false));
                if step == 3 {
                    accounts.push(AccountMeta::new(lp_token_custody, false));
                }
                if step == 1 {
                    accounts.push(AccountMeta::new(
                        vault
                            .fees_account_a
                            .ok_or(ProgramError::UninitializedAccount)?,
                        false,
                    ));
                }

                if step == 2 || step == 3 {
                    match pool.route {
                        PoolRoute::Orca {
                            amm_id,
                            amm_authority,
                            fees_account,
                        } => {
                            accounts.push(AccountMeta::new(token_a_custody, false));
                            accounts.push(AccountMeta::new(
                                token_b_custody.or_else(|| Some(zero::id())).unwrap(),
                                false,
                            ));
                            accounts.push(AccountMeta::new_readonly(pool.pool_program_id, false));
                            accounts.push(AccountMeta::new(
                                pool.token_a_account
                                    .ok_or(ProgramError::UninitializedAccount)?,
                                false,
                            ));
                            accounts.push(AccountMeta::new(
                                pool.token_b_account
                                    .ok_or(ProgramError::UninitializedAccount)?,
                                false,
                            ));
                            accounts.push(AccountMeta::new(
                                lp_token.ok_or(ProgramError::UninitializedAccount)?.mint,
                                false,
                            ));
                            accounts.push(AccountMeta::new(amm_id, false));
                            accounts.push(AccountMeta::new_readonly(amm_authority, false));
                            if step == 2 {
                                accounts.push(AccountMeta::new(fees_account, false));
                                if let Some(rdex_pool_ref) = reward_exchange_pool_ref {
                                    let rdex_pool = self.get_pool_by_ref(&rdex_pool_ref)?;
                                    let rdex_lp_token =
                                        self.get_token_by_ref_from_cache(&rdex_pool.lp_token_ref)?;
                                    match rdex_pool.route {
                                        PoolRoute::Orca {
                                            amm_id: rdex_amm_id,
                                            amm_authority: rdex_amm_authority,
                                            fees_account: rdex_fees_account,
                                        } => {
                                            accounts.push(AccountMeta::new(
                                                rdex_pool
                                                    .token_a_account
                                                    .ok_or(ProgramError::UninitializedAccount)?,
                                                false,
                                            ));
                                            accounts.push(AccountMeta::new(
                                                rdex_pool
                                                    .token_b_account
                                                    .ok_or(ProgramError::UninitializedAccount)?,
                                                false,
                                            ));
                                            accounts.push(AccountMeta::new(
                                                rdex_lp_token
                                                    .ok_or(ProgramError::UninitializedAccount)?
                                                    .mint,
                                                false,
                                            ));
                                            accounts.push(AccountMeta::new(rdex_amm_id, false));
                                            accounts.push(AccountMeta::new_readonly(
                                                rdex_amm_authority,
                                                false,
                                            ));
                                            accounts
                                                .push(AccountMeta::new(rdex_fees_account, false));
                                        }
                                        _ => {
                                            unreachable!();
                                        }
                                    }
                                } else {
                                    for _ in 0..6 {
                                        accounts.push(AccountMeta::new_readonly(zero::id(), false));
                                    }
                                }
                                accounts.push(AccountMeta::new_readonly(
                                    sysvar::instructions::id(),
                                    false,
                                ));
                            }
                        }
                        _ => {
                            unreachable!();
                        }
                    }
                }

                // fill in farm related accounts
                if step == 1 || step == 3 {
                    match farm.route {
                        FarmRoute::Orca {
                            farm_id,
                            farm_authority,
                            farm_token_ref,
                            base_token_vault,
                            reward_token_vault,
                        } => {
                            let farm_lp_token =
                                self.get_token_by_ref_from_cache(&Some(farm_token_ref))?;

                            accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
                            accounts.push(AccountMeta::new(vault_stake_info, false));
                            if step == 3 {
                                accounts.push(AccountMeta::new(
                                    vault_stake_custody
                                        .ok_or(ProgramError::UninitializedAccount)?,
                                    false,
                                ));
                            }
                            accounts.push(AccountMeta::new(farm_id, false));
                            accounts.push(AccountMeta::new_readonly(farm_authority, false));
                            if step == 3 {
                                accounts.push(AccountMeta::new(
                                    farm_lp_token
                                        .ok_or(ProgramError::UninitializedAccount)?
                                        .mint,
                                    false,
                                ));
                            }
                            accounts.push(AccountMeta::new(base_token_vault, false));
                            accounts.push(AccountMeta::new(reward_token_vault, false));
                        }
                        _ => {
                            unreachable!();
                        }
                    }
                }
            }
            VaultStrategy::DynamicHedge { .. } => {
                unreachable!();
            }
        }

        Ok((accounts, data))
    }
}
