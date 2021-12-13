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
    pub fn get_stc_user_init_accounts_raydium(
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
            AccountMeta::new_readonly(*wallet_address, true),
            AccountMeta::new_readonly(vault_ref, false),
            AccountMeta::new(vault.info_account, false),
            AccountMeta::new(
                self.get_vault_user_info_account(wallet_address, vault_name)?,
                false,
            ),
            AccountMeta::new_readonly(system_program::id(), false),
        ];
        Ok((accounts, data))
    }

    /// Returns accounts and data for adding liquidity to the Vault
    pub fn get_stc_add_liquidity_accounts_raydium(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
        max_token_a_ui_amount: f64,
        max_token_b_ui_amount: f64,
    ) -> Result<(Vec<AccountMeta>, Vec<u8>), FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;
        let vault_token = self.get_token_by_ref_from_cache(&Some(vault.vault_token_ref))?;

        // fill in accounts and instruction data
        let data;
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
                pool_id_ref,
                farm_id_ref,
                lp_token_custody,
                token_a_custody: _,
                token_b_custody: _,
                token_a_reward_custody,
                token_b_reward_custody,
                vault_stake_info,
            } => {
                let pool = self.get_pool_by_ref(&pool_id_ref)?;
                let farm = self.get_farm_by_ref(&farm_id_ref)?;

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
                let user_vt_token_account = self.get_token_account(wallet_address, &vault_token);

                // fill in pool related accounts
                match pool.route {
                    PoolRoute::Raydium {
                        amm_id,
                        amm_authority,
                        amm_open_orders,
                        amm_target,
                        pool_withdraw_queue: _,
                        pool_temp_lp_token_account: _,
                        serum_program_id: _,
                        serum_market,
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
                        accounts.push(AccountMeta::new(
                            user_vt_token_account.ok_or(ProgramError::UninitializedAccount)?,
                            false,
                        ));
                        accounts.push(AccountMeta::new(token_a_reward_custody, false));
                        accounts.push(AccountMeta::new(
                            token_b_reward_custody.or_else(|| Some(zero::id())).unwrap(),
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
                        accounts.push(AccountMeta::new_readonly(amm_open_orders, false));
                        accounts.push(AccountMeta::new(amm_target, false));
                        accounts.push(AccountMeta::new_readonly(serum_market, false));
                    }
                    _ => {
                        unreachable!();
                    }
                }

                // fill in farm related accounts
                match farm.route {
                    FarmRoute::Raydium {
                        farm_id,
                        farm_authority,
                        farm_lp_token_account,
                        farm_reward_token_a_account,
                        farm_reward_token_b_account,
                    } => {
                        accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
                        accounts.push(AccountMeta::new(vault_stake_info, false));
                        accounts.push(AccountMeta::new(farm_id, false));
                        accounts.push(AccountMeta::new_readonly(farm_authority, false));

                        accounts.push(AccountMeta::new(farm_lp_token_account, false));
                        accounts.push(AccountMeta::new(farm_reward_token_a_account, false));
                        accounts.push(AccountMeta::new(
                            farm_reward_token_b_account
                                .or_else(|| Some(zero::id()))
                                .unwrap(),
                            false,
                        ));
                        accounts.push(AccountMeta::new_readonly(sysvar::clock::id(), false));
                    }
                    _ => {
                        unreachable!();
                    }
                }

                data = VaultInstruction::AddLiquidity {
                    max_token_a_amount: self
                        .to_token_amount_option(max_token_a_ui_amount, &token_a)?,
                    max_token_b_amount: self
                        .to_token_amount_option(max_token_b_ui_amount, &token_b)?,
                }
                .to_vec()?;
            }
            VaultStrategy::DynamicHedge { .. } => {
                unreachable!();
            }
        }
        Ok((accounts, data))
    }

    /// Returns accounts and data for unlocking liquidity in the Vault
    pub fn get_stc_unlock_liquidity_accounts_raydium(
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
                pool_id_ref,
                farm_id_ref,
                lp_token_custody,
                token_a_custody: _,
                token_b_custody: _,
                token_a_reward_custody,
                token_b_reward_custody,
                vault_stake_info,
            } => {
                let pool = self.get_pool_by_ref(&pool_id_ref)?;
                let farm = self.get_farm_by_ref(&farm_id_ref)?;

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
                accounts.push(AccountMeta::new(
                    token_b_reward_custody.or_else(|| Some(zero::id())).unwrap(),
                    false,
                ));
                accounts.push(AccountMeta::new(lp_token_custody, false));

                // fill in farm related accounts
                match farm.route {
                    FarmRoute::Raydium {
                        farm_id,
                        farm_authority,
                        farm_lp_token_account,
                        farm_reward_token_a_account,
                        farm_reward_token_b_account,
                    } => {
                        accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
                        accounts.push(AccountMeta::new(vault_stake_info, false));
                        accounts.push(AccountMeta::new(farm_id, false));
                        accounts.push(AccountMeta::new_readonly(farm_authority, false));
                        accounts.push(AccountMeta::new(farm_lp_token_account, false));
                        accounts.push(AccountMeta::new(farm_reward_token_a_account, false));
                        accounts.push(AccountMeta::new(
                            farm_reward_token_b_account
                                .or_else(|| Some(zero::id()))
                                .unwrap(),
                            false,
                        ));
                        accounts.push(AccountMeta::new_readonly(sysvar::clock::id(), false));
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
    pub fn get_stc_remove_liquidity_accounts_raydium(
        &self,
        wallet_address: &Pubkey,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<(Vec<AccountMeta>, Vec<u8>), FarmClientError> {
        // get vault info
        let vault = self.get_vault(vault_name)?;
        let vault_ref = self.get_vault_ref(vault_name)?;
        // fill in accounts and instruction data
        let data;
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
        match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards {
                pool_id_ref,
                farm_id_ref,
                lp_token_custody,
                token_a_custody,
                token_b_custody,
                ..
            } => {
                let pool = self.get_pool_by_ref(&pool_id_ref)?;
                let farm = self.get_farm_by_ref(&farm_id_ref)?;

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
                    PoolRoute::Raydium {
                        amm_id,
                        amm_authority,
                        amm_open_orders,
                        amm_target,
                        pool_withdraw_queue,
                        pool_temp_lp_token_account,
                        serum_program_id,
                        serum_market,
                        serum_coin_vault_account,
                        serum_pc_vault_account,
                        serum_vault_signer,
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
                        accounts.push(AccountMeta::new(token_a_custody, false));
                        accounts.push(AccountMeta::new(
                            token_b_custody.or_else(|| Some(zero::id())).unwrap(),
                            false,
                        ));
                        accounts.push(AccountMeta::new(lp_token_custody, false));
                        accounts.push(AccountMeta::new_readonly(pool.pool_program_id, false));
                        accounts.push(AccountMeta::new(pool_withdraw_queue, false));
                        accounts.push(AccountMeta::new(pool_temp_lp_token_account, false));
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
                        accounts.push(AccountMeta::new(amm_open_orders, false));
                        accounts.push(AccountMeta::new(amm_target, false));
                        accounts.push(AccountMeta::new(serum_market, false));
                        accounts.push(AccountMeta::new_readonly(serum_program_id, false));
                        accounts.push(AccountMeta::new(serum_coin_vault_account, false));
                        accounts.push(AccountMeta::new(serum_pc_vault_account, false));
                        accounts.push(AccountMeta::new_readonly(serum_vault_signer, false));
                    }
                    _ => {
                        unreachable!();
                    }
                }

                data = VaultInstruction::RemoveLiquidity {
                    amount: self.to_token_amount_option(ui_amount, &lp_token)?,
                }
                .to_vec()?;
            }
            VaultStrategy::DynamicHedge { .. } => {
                unreachable!();
            }
        }
        Ok((accounts, data))
    }

    /// Returns accounts and data for a Vault Init Instruction
    pub fn get_stc_init_accounts_raydium(
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
        accounts.push(AccountMeta::new(vault.vault_authority, false));
        accounts.push(AccountMeta::new_readonly(vault.vault_program_id, false));
        accounts.push(AccountMeta::new_readonly(system_program::id(), false));
        accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
        accounts.push(AccountMeta::new_readonly(sysvar::rent::id(), false));

        match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards {
                pool_id_ref,
                farm_id_ref,
                lp_token_custody,
                token_a_custody,
                token_b_custody,
                token_a_reward_custody,
                token_b_reward_custody,
                vault_stake_info,
            } => {
                // get pools
                let pool = self.get_pool_by_ref(&pool_id_ref)?;
                let farm = self.get_farm_by_ref(&farm_id_ref)?;
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
                let token_a_reward = self
                    .get_token_by_ref_from_cache(&farm.reward_token_a_ref)?
                    .unwrap();
                let token_b_reward = self.get_token_by_ref_from_cache(&farm.reward_token_b_ref)?;

                accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
                accounts.push(AccountMeta::new(vault_token.mint, false));
                accounts.push(AccountMeta::new_readonly(vault.vault_token_ref, false));
                if farm.version >= 4 {
                    accounts.push(AccountMeta::new(zero::id(), false));
                    accounts.push(AccountMeta::new(vault_stake_info, false));
                } else {
                    accounts.push(AccountMeta::new(vault_stake_info, false));
                    accounts.push(AccountMeta::new(zero::id(), false));
                }
                accounts.push(AccountMeta::new(
                    vault.fees_account_a.or_else(|| Some(zero::id())).unwrap(),
                    false,
                ));
                accounts.push(AccountMeta::new(
                    vault.fees_account_b.or_else(|| Some(zero::id())).unwrap(),
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

                accounts.push(AccountMeta::new(token_a_reward_custody, false));
                accounts.push(AccountMeta::new(
                    token_b_reward_custody.or_else(|| Some(zero::id())).unwrap(),
                    false,
                ));
                accounts.push(AccountMeta::new(token_a_reward.mint, false));
                if let Some(token) = token_b_reward {
                    accounts.push(AccountMeta::new(token.mint, false));
                } else {
                    accounts.push(AccountMeta::new(zero::id(), false));
                }
            }
            VaultStrategy::DynamicHedge { .. } => {
                unreachable!();
            }
        }

        Ok((accounts, data))
    }

    /// Returns accounts and data for a Vault Shutdown Instruction
    pub fn get_stc_shutdown_accounts_raydium(
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
        ];

        Ok((accounts, data))
    }

    /// Returns accounts and data for a Vault Crank Instruction
    pub fn get_stc_crank_accounts_raydium(
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
                pool_id_ref,
                farm_id_ref,
                lp_token_custody,
                token_a_custody,
                token_b_custody,
                token_a_reward_custody,
                token_b_reward_custody,
                vault_stake_info,
            } => {
                let pool = self.get_pool_by_ref(&pool_id_ref)?;
                let farm = self.get_farm_by_ref(&farm_id_ref)?;

                // get tokens info
                let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;
                let farm_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;
                assert_eq!(farm_token, lp_token);

                accounts.push(AccountMeta::new(token_a_reward_custody, false));
                accounts.push(AccountMeta::new(
                    token_b_reward_custody.or_else(|| Some(zero::id())).unwrap(),
                    false,
                ));
                if step != 2 {
                    accounts.push(AccountMeta::new(lp_token_custody, false));
                }
                if step == 1 {
                    accounts.push(AccountMeta::new(
                        vault
                            .fees_account_a
                            .ok_or(ProgramError::UninitializedAccount)?,
                        false,
                    ));
                    accounts.push(AccountMeta::new(
                        vault.fees_account_b.or_else(|| Some(zero::id())).unwrap(),
                        false,
                    ));
                }

                if step == 2 || step == 3 {
                    match pool.route {
                        PoolRoute::Raydium {
                            amm_id,
                            amm_authority,
                            amm_open_orders,
                            amm_target,
                            pool_withdraw_queue: _,
                            pool_temp_lp_token_account: _,
                            serum_program_id,
                            serum_market,
                            serum_coin_vault_account,
                            serum_pc_vault_account,
                            serum_vault_signer,
                            serum_bids,
                            serum_asks,
                            serum_event_queue,
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
                            if step == 3 {
                                accounts.push(AccountMeta::new(
                                    lp_token.ok_or(ProgramError::UninitializedAccount)?.mint,
                                    false,
                                ));
                            }
                            accounts.push(AccountMeta::new(amm_id, false));
                            accounts.push(AccountMeta::new_readonly(amm_authority, false));
                            accounts.push(AccountMeta::new(amm_open_orders, false));
                            accounts.push(AccountMeta::new(amm_target, false));
                            accounts.push(AccountMeta::new(serum_market, false));

                            if step == 2 {
                                accounts.push(AccountMeta::new_readonly(serum_program_id, false));
                                accounts.push(AccountMeta::new(serum_coin_vault_account, false));
                                accounts.push(AccountMeta::new(serum_pc_vault_account, false));
                                accounts.push(AccountMeta::new(serum_vault_signer, false));
                                accounts.push(AccountMeta::new(
                                    serum_bids.ok_or(ProgramError::UninitializedAccount)?,
                                    false,
                                ));
                                accounts.push(AccountMeta::new(
                                    serum_asks.ok_or(ProgramError::UninitializedAccount)?,
                                    false,
                                ));
                                accounts.push(AccountMeta::new(
                                    serum_event_queue.ok_or(ProgramError::UninitializedAccount)?,
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
                        FarmRoute::Raydium {
                            farm_id,
                            farm_authority,
                            farm_lp_token_account,
                            farm_reward_token_a_account,
                            farm_reward_token_b_account,
                        } => {
                            accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
                            accounts.push(AccountMeta::new(vault_stake_info, false));
                            accounts.push(AccountMeta::new(farm_id, false));
                            accounts.push(AccountMeta::new_readonly(farm_authority, false));

                            accounts.push(AccountMeta::new(farm_lp_token_account, false));
                            accounts.push(AccountMeta::new(farm_reward_token_a_account, false));
                            accounts.push(AccountMeta::new(
                                farm_reward_token_b_account
                                    .or_else(|| Some(zero::id()))
                                    .unwrap(),
                                false,
                            ));
                            accounts.push(AccountMeta::new_readonly(sysvar::clock::id(), false));
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
