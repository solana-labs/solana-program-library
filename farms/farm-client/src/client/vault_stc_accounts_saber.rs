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
    pub fn get_stc_user_init_accounts_saber(
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
    pub fn get_stc_add_liquidity_accounts_saber(
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
        let data;
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
        match vault.strategy {
            VaultStrategy::StakeLpCompoundRewards {
                pool_id_ref,
                farm_id_ref,
                lp_token_custody,
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
                let user_lp_token_account = self.get_token_account(wallet_address, &lp_token);
                // fill in pool related accounts
                match pool.route {
                    PoolRoute::Saber {
                        swap_account,
                        swap_authority,
                        wrapped_token_a_ref,
                        wrapped_token_b_ref,
                        ..
                    } => {
                        let wrapped_token_a =
                            self.get_token_by_ref_from_cache(&wrapped_token_a_ref)?;
                        let user_token_a_account = if wrapped_token_a.is_some() {
                            self.get_token_account(wallet_address, &wrapped_token_a)
                        } else {
                            let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
                            self.get_token_account(wallet_address, &token_a)
                        };
                        let wrapped_token_b =
                            self.get_token_by_ref_from_cache(&wrapped_token_b_ref)?;
                        let user_token_b_account = if wrapped_token_b.is_some() {
                            self.get_token_account(wallet_address, &wrapped_token_b)
                        } else {
                            let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;
                            self.get_token_account(wallet_address, &token_b)
                        };

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
                        accounts.push(AccountMeta::new_readonly(sysvar::clock::id(), false));
                        accounts.push(AccountMeta::new_readonly(swap_account, false));
                        accounts.push(AccountMeta::new_readonly(swap_authority, false));
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

    /// Returns accounts and data for locking liquidity in the Vault
    pub fn get_stc_lock_liquidity_accounts_saber(
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
                token_a_reward_custody: _,
                token_b_reward_custody: _,
                vault_stake_info,
            } => {
                let pool = self.get_pool_by_ref(&pool_id_ref)?;
                let farm = self.get_farm_by_ref(&farm_id_ref)?;

                // get tokens info
                let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;
                let farm_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;
                assert_eq!(farm_token, lp_token);

                let user_vt_token_account = self.get_token_account(wallet_address, &vault_token);

                // fill in farm related accounts
                match farm.route {
                    FarmRoute::Saber {
                        quarry, rewarder, ..
                    } => {
                        let vault_miner_account = self
                            .get_token_account(&vault_stake_info, &lp_token)
                            .ok_or(ProgramError::UninitializedAccount)?;

                        accounts.push(AccountMeta::new(
                            user_vt_token_account.ok_or(ProgramError::UninitializedAccount)?,
                            false,
                        ));
                        accounts.push(AccountMeta::new(lp_token_custody, false));
                        accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
                        accounts.push(AccountMeta::new(vault_stake_info, false));
                        accounts.push(AccountMeta::new(vault_miner_account, false));
                        accounts.push(AccountMeta::new(quarry, false));
                        accounts.push(AccountMeta::new_readonly(rewarder, false));
                    }
                    _ => {
                        unreachable!();
                    }
                }

                data = VaultInstruction::LockLiquidity {
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

    /// Returns accounts and data for removing liquidity from the Vault
    pub fn get_stc_remove_liquidity_accounts_saber(
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
                token_a_reward_custody: _,
                token_b_reward_custody: _,
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

                // fill in pool related accounts
                match pool.route {
                    PoolRoute::Saber {
                        swap_account,
                        swap_authority,
                        fees_account_a,
                        fees_account_b,
                        wrapped_token_a_ref,
                        wrapped_token_b_ref,
                        ..
                    } => {
                        let wrapped_token_a =
                            self.get_token_by_ref_from_cache(&wrapped_token_a_ref)?;
                        let user_token_a_account = if wrapped_token_a.is_some() {
                            self.get_token_account(wallet_address, &wrapped_token_a)
                        } else {
                            let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
                            self.get_token_account(wallet_address, &token_a)
                        };
                        let wrapped_token_b =
                            self.get_token_by_ref_from_cache(&wrapped_token_b_ref)?;
                        let user_token_b_account = if wrapped_token_b.is_some() {
                            self.get_token_account(wallet_address, &wrapped_token_b)
                        } else {
                            let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;
                            self.get_token_account(wallet_address, &token_b)
                        };

                        accounts.push(AccountMeta::new(
                            user_token_a_account.ok_or(ProgramError::UninitializedAccount)?,
                            false,
                        ));
                        accounts.push(AccountMeta::new(
                            user_token_b_account.ok_or(ProgramError::UninitializedAccount)?,
                            false,
                        ));
                        accounts.push(AccountMeta::new(
                            user_vt_token_account.ok_or(ProgramError::UninitializedAccount)?,
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
                        accounts.push(AccountMeta::new_readonly(swap_account, false));
                        accounts.push(AccountMeta::new_readonly(swap_authority, false));
                        accounts.push(AccountMeta::new(fees_account_a, false));
                        accounts.push(AccountMeta::new(fees_account_b, false));
                    }
                    _ => {
                        unreachable!();
                    }
                }

                // fill in farm related accounts
                match farm.route {
                    FarmRoute::Saber {
                        quarry, rewarder, ..
                    } => {
                        let vault_miner_account = self
                            .get_token_account(&vault_stake_info, &lp_token)
                            .ok_or(ProgramError::UninitializedAccount)?;

                        accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
                        accounts.push(AccountMeta::new(vault_stake_info, false));
                        accounts.push(AccountMeta::new(vault_miner_account, false));
                        accounts.push(AccountMeta::new(quarry, false));
                        accounts.push(AccountMeta::new_readonly(rewarder, false));
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
    pub fn get_stc_init_accounts_saber(
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
        let mut accounts = vec![AccountMeta::new(*admin_address, true)];

        // general accounts
        accounts.push(AccountMeta::new_readonly(vault_ref, false));
        accounts.push(AccountMeta::new(vault.info_account, false));
        accounts.push(AccountMeta::new(vault.vault_authority, false));
        accounts.push(AccountMeta::new_readonly(vault.vault_program_id, false));
        accounts.push(AccountMeta::new_readonly(system_program::id(), false));
        accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
        accounts.push(AccountMeta::new_readonly(
            spl_associated_token_account::id(),
            false,
        ));
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
                let usdc_token = self.get_token("USDC")?;
                let token_a_usdc = if token_a.mint == usdc_token.mint {
                    true
                } else if token_b.mint == usdc_token.mint {
                    false
                } else {
                    return Err(FarmClientError::ValueError(
                        "Only USDC pools are supported".to_string(),
                    ));
                };
                let lp_token = self
                    .get_token_by_ref_from_cache(&pool.lp_token_ref)?
                    .unwrap();
                let token_a_reward = self
                    .get_token_by_ref_from_cache(&farm.reward_token_a_ref)?
                    .unwrap();
                let token_b_reward = self.get_token_by_ref_from_cache(&farm.reward_token_b_ref)?;
                let wrapped_token_mint = match pool.route {
                    PoolRoute::Saber {
                        wrapped_token_a_ref,
                        wrapped_token_b_ref,
                        ..
                    } => {
                        let wrapped_token_a =
                            self.get_token_by_ref_from_cache(&wrapped_token_a_ref)?;
                        let wrapped_token_b =
                            self.get_token_by_ref_from_cache(&wrapped_token_b_ref)?;
                        if wrapped_token_a.is_some() && wrapped_token_b.is_some() {
                            // no such pools
                            unreachable!();
                        } else if wrapped_token_a.is_some() {
                            wrapped_token_a.unwrap().mint
                        } else if let Some(token) = wrapped_token_b {
                            token.mint
                        } else {
                            zero::id()
                        }
                    }
                    _ => {
                        unreachable!()
                    }
                };

                let vault_miner_account = self
                    .get_token_account(&vault_stake_info, &Some(lp_token))
                    .ok_or(ProgramError::UninitializedAccount)?;

                accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
                accounts.push(AccountMeta::new(vault_token.mint, false));
                accounts.push(AccountMeta::new_readonly(vault.vault_token_ref, false));
                accounts.push(AccountMeta::new(vault_stake_info, false));
                accounts.push(AccountMeta::new(vault_miner_account, false));
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
                accounts.push(AccountMeta::new(usdc_token.mint, false));
                if token_a_usdc {
                    accounts.push(AccountMeta::new(token_b.mint, false));
                } else {
                    accounts.push(AccountMeta::new(token_a.mint, false));
                }
                accounts.push(AccountMeta::new(wrapped_token_mint, false));
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
                // fill in farm related accounts
                match farm.route {
                    FarmRoute::Saber {
                        quarry, rewarder, ..
                    } => {
                        accounts.push(AccountMeta::new(quarry, false));
                        accounts.push(AccountMeta::new(rewarder, false));
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

    /// Returns accounts and data for a Vault Shutdown Instruction
    pub fn get_stc_shutdown_accounts_saber(
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
    pub fn get_stc_crank_accounts_saber(
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
                let sbr_token = self.get_token_by_ref_from_cache(&farm.reward_token_a_ref)?;
                let iou_token = self.get_token_by_ref_from_cache(&farm.reward_token_b_ref)?;
                let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
                let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;
                let farm_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;
                assert_eq!(farm_token, lp_token);
                let (is_token_a_wrapped, is_token_b_wrapped) =
                    self.pool_has_saber_wrapped_tokens(&pool.name)?;
                let usdc_mint = self.get_token("USDC")?.mint;

                match farm.route {
                    FarmRoute::Saber {
                        quarry,
                        rewarder,
                        redeemer,
                        redeemer_program,
                        minter,
                        mint_wrapper,
                        mint_wrapper_program,
                        iou_fees_account,
                        sbr_vault,
                        mint_proxy_program,
                        mint_proxy_authority,
                        mint_proxy_state,
                        minter_info,
                    } => match step {
                        1 => {
                            accounts.push(AccountMeta::new(
                                token_b_reward_custody.ok_or(ProgramError::UninitializedAccount)?,
                                false,
                            ));
                            accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
                            accounts.push(AccountMeta::new(vault_stake_info, false));
                            accounts.push(AccountMeta::new(mint_wrapper, false));
                            accounts.push(AccountMeta::new_readonly(mint_wrapper_program, false));
                            accounts.push(AccountMeta::new(minter, false));
                            accounts.push(AccountMeta::new(
                                iou_token.ok_or(ProgramError::UninitializedAccount)?.mint,
                                false,
                            ));
                            accounts.push(AccountMeta::new(iou_fees_account, false));
                            accounts.push(AccountMeta::new(quarry, false));
                            accounts.push(AccountMeta::new_readonly(rewarder, false));
                            accounts.push(AccountMeta::new(zero::id(), false));
                        }
                        2 => {
                            accounts.push(AccountMeta::new(token_a_reward_custody, false));
                            accounts.push(AccountMeta::new(
                                token_b_reward_custody.ok_or(ProgramError::UninitializedAccount)?,
                                false,
                            ));
                            accounts.push(AccountMeta::new(
                                vault
                                    .fees_account_a
                                    .ok_or(ProgramError::UninitializedAccount)?,
                                false,
                            ));
                            accounts.push(AccountMeta::new_readonly(redeemer, false));
                            accounts.push(AccountMeta::new_readonly(redeemer_program, false));
                            accounts.push(AccountMeta::new(
                                sbr_token.ok_or(ProgramError::UninitializedAccount)?.mint,
                                false,
                            ));
                            accounts.push(AccountMeta::new(
                                iou_token.ok_or(ProgramError::UninitializedAccount)?.mint,
                                false,
                            ));
                            accounts.push(AccountMeta::new(sbr_vault, false));
                            accounts.push(AccountMeta::new_readonly(mint_proxy_program, false));
                            accounts.push(AccountMeta::new_readonly(mint_proxy_authority, false));
                            accounts.push(AccountMeta::new_readonly(mint_proxy_state, false));
                            accounts.push(AccountMeta::new(minter_info, false));
                        }
                        3 => {
                            accounts.push(AccountMeta::new(token_a_reward_custody, false));
                            accounts.push(AccountMeta::new(token_a_custody, false));
                            accounts.push(AccountMeta::new(
                                token_b_custody.or_else(|| Some(zero::id())).unwrap(),
                                false,
                            ));

                            match pool.route {
                                PoolRoute::Saber {
                                    decimal_wrapper_program,
                                    wrapped_token_a_ref,
                                    wrapped_token_a_vault,
                                    decimal_wrapper_token_a,
                                    wrapped_token_b_ref,
                                    wrapped_token_b_vault,
                                    decimal_wrapper_token_b,
                                    ..
                                } => {
                                    let (wrapped_token_mint, wrapped_token_vault, decimal_wrapper) =
                                        if is_token_a_wrapped {
                                            let wrapped_token_a = self
                                                .get_token_by_ref_from_cache(
                                                    &wrapped_token_a_ref,
                                                )?;
                                            (
                                                wrapped_token_a
                                                    .ok_or(ProgramError::UninitializedAccount)?
                                                    .mint,
                                                wrapped_token_a_vault
                                                    .ok_or(ProgramError::UninitializedAccount)?,
                                                decimal_wrapper_token_a
                                                    .ok_or(ProgramError::UninitializedAccount)?,
                                            )
                                        } else if is_token_b_wrapped {
                                            let wrapped_token_b = self
                                                .get_token_by_ref_from_cache(
                                                    &wrapped_token_b_ref,
                                                )?;
                                            (
                                                wrapped_token_b
                                                    .ok_or(ProgramError::UninitializedAccount)?
                                                    .mint,
                                                wrapped_token_b_vault
                                                    .ok_or(ProgramError::UninitializedAccount)?,
                                                decimal_wrapper_token_b
                                                    .ok_or(ProgramError::UninitializedAccount)?,
                                            )
                                        } else {
                                            (zero::id(), zero::id(), zero::id())
                                        };

                                    accounts.push(AccountMeta::new_readonly(usdc_mint, false));
                                    accounts.push(AccountMeta::new(wrapped_token_mint, false));
                                    accounts.push(AccountMeta::new(wrapped_token_vault, false));
                                    accounts
                                        .push(AccountMeta::new_readonly(decimal_wrapper, false));
                                    accounts.push(AccountMeta::new_readonly(
                                        decimal_wrapper_program,
                                        false,
                                    ));
                                }
                                _ => {
                                    unreachable!();
                                }
                            }

                            let usdc_pool = self.get_pool("RDM.SBR-USDC")?;
                            match usdc_pool.route {
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
                                    accounts.push(AccountMeta::new_readonly(
                                        usdc_pool.pool_program_id,
                                        false,
                                    ));
                                    accounts.push(AccountMeta::new(
                                        usdc_pool
                                            .token_a_account
                                            .ok_or(ProgramError::UninitializedAccount)?,
                                        false,
                                    ));
                                    accounts.push(AccountMeta::new(
                                        usdc_pool
                                            .token_b_account
                                            .ok_or(ProgramError::UninitializedAccount)?,
                                        false,
                                    ));
                                    accounts.push(AccountMeta::new(amm_id, false));
                                    accounts.push(AccountMeta::new_readonly(amm_authority, false));
                                    accounts.push(AccountMeta::new(amm_open_orders, false));
                                    accounts.push(AccountMeta::new(amm_target, false));
                                    accounts.push(AccountMeta::new(serum_market, false));
                                    accounts
                                        .push(AccountMeta::new_readonly(serum_program_id, false));
                                    accounts
                                        .push(AccountMeta::new(serum_coin_vault_account, false));
                                    accounts.push(AccountMeta::new(serum_pc_vault_account, false));
                                    accounts
                                        .push(AccountMeta::new_readonly(serum_vault_signer, false));
                                    accounts.push(AccountMeta::new(
                                        serum_bids.ok_or(ProgramError::UninitializedAccount)?,
                                        false,
                                    ));
                                    accounts.push(AccountMeta::new(
                                        serum_asks.ok_or(ProgramError::UninitializedAccount)?,
                                        false,
                                    ));
                                    accounts.push(AccountMeta::new(
                                        serum_event_queue
                                            .ok_or(ProgramError::UninitializedAccount)?,
                                        false,
                                    ));
                                }
                                _ => {
                                    unreachable!();
                                }
                            }
                        }
                        4 => match pool.route {
                            PoolRoute::Saber {
                                swap_account,
                                swap_authority,
                                ..
                            } => {
                                let usdc_token = self.get_token("USDC")?;
                                if token_a.ok_or(ProgramError::UninitializedAccount)?.mint
                                    != usdc_token.mint
                                {
                                    accounts.push(AccountMeta::new(
                                        vault
                                            .fees_account_b
                                            .ok_or(ProgramError::UninitializedAccount)?,
                                        false,
                                    ));
                                }
                                if is_token_a_wrapped || is_token_b_wrapped {
                                    accounts.push(AccountMeta::new(
                                        token_b_custody
                                            .ok_or(ProgramError::UninitializedAccount)?,
                                        false,
                                    ));
                                } else {
                                    accounts.push(AccountMeta::new(token_a_custody, false));
                                }
                                if token_a.ok_or(ProgramError::UninitializedAccount)?.mint
                                    == usdc_token.mint
                                {
                                    accounts.push(AccountMeta::new(
                                        vault
                                            .fees_account_b
                                            .ok_or(ProgramError::UninitializedAccount)?,
                                        false,
                                    ));
                                }
                                accounts.push(AccountMeta::new(lp_token_custody, false));
                                accounts
                                    .push(AccountMeta::new_readonly(pool.pool_program_id, false));
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
                                accounts
                                    .push(AccountMeta::new_readonly(sysvar::clock::id(), false));
                                accounts.push(AccountMeta::new_readonly(swap_account, false));
                                accounts.push(AccountMeta::new_readonly(swap_authority, false));
                            }
                            _ => {
                                unreachable!();
                            }
                        },
                        5 => {
                            let vault_miner_account = self
                                .get_token_account(&vault_stake_info, &lp_token)
                                .ok_or(ProgramError::UninitializedAccount)?;

                            accounts.push(AccountMeta::new(lp_token_custody, false));
                            accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
                            accounts.push(AccountMeta::new(vault_stake_info, false));
                            accounts.push(AccountMeta::new(vault_miner_account, false));
                            accounts.push(AccountMeta::new(quarry, false));
                            accounts.push(AccountMeta::new_readonly(rewarder, false));
                        }
                        _ => {
                            panic!("Crank step must be 1-5");
                        }
                    },
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
}
