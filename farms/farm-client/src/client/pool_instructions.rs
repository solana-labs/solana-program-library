//! Solana Farm Client Pool Instructions

use {
    crate::error::FarmClientError,
    solana_farm_sdk::{
        instruction::amm::AmmInstruction, pool::PoolRoute, program::account, token::TokenSelector,
    },
    solana_sdk::{instruction::Instruction, program_error::ProgramError, pubkey::Pubkey},
};

use super::FarmClient;

impl FarmClient {
    /// Creates a new Instruction for adding liquidity to the Pool.
    /// If one of the token amounts is 0 and pool requires both tokens,
    /// amount will be autocalculated based on the current pool price.
    pub fn new_instruction_add_liquidity_pool(
        &self,
        wallet_address: &Pubkey,
        pool_name: &str,
        max_token_a_ui_amount: f64,
        max_token_b_ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get pool info
        let pool = self.get_pool(pool_name)?;

        // get tokens info
        let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;

        // convert amounts if wrapped tokens are used
        let mut max_token_a_amount =
            self.to_token_amount_option(max_token_a_ui_amount, &token_a)?;
        let mut max_token_b_amount =
            self.to_token_amount_option(max_token_b_ui_amount, &token_b)?;
        if let PoolRoute::Saber {
            wrapped_token_a_ref,
            wrapped_token_b_ref,
            ..
        } = pool.route
        {
            if let Some(token_ref) = wrapped_token_a_ref {
                let underlying_decimals =
                    token_a.ok_or(ProgramError::UninitializedAccount)?.decimals;
                let wrapped_decimals = self.get_token_by_ref(&token_ref)?.decimals;
                max_token_a_amount = account::to_amount_with_new_decimals(
                    max_token_a_amount,
                    underlying_decimals,
                    wrapped_decimals,
                )?;
            }
            if let Some(token_ref) = wrapped_token_b_ref {
                let underlying_decimals =
                    token_b.ok_or(ProgramError::UninitializedAccount)?.decimals;
                let wrapped_decimals = self.get_token_by_ref(&token_ref)?.decimals;
                max_token_b_amount = account::to_amount_with_new_decimals(
                    max_token_b_amount,
                    underlying_decimals,
                    wrapped_decimals,
                )?;
            }
        }

        // fill in instruction data
        let data = AmmInstruction::AddLiquidity {
            max_token_a_amount,
            max_token_b_amount,
        }
        .to_vec()?;

        let accounts = match pool.route {
            PoolRoute::Raydium { .. } => {
                self.get_add_liquidity_accounts_raydium(wallet_address, pool_name)?
            }
            PoolRoute::Saber { .. } => {
                self.get_add_liquidity_accounts_saber(wallet_address, pool_name)?
            }
            PoolRoute::Orca { .. } => {
                self.get_add_liquidity_accounts_orca(wallet_address, pool_name)?
            }
        };

        Ok(Instruction {
            program_id: pool.router_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for removing liquidity from the Pool
    pub fn new_instruction_remove_liquidity_pool(
        &self,
        wallet_address: &Pubkey,
        pool_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get pool info
        let pool = self.get_pool(pool_name)?;

        // get tokens info
        let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;

        // fill in instruction data
        let data = AmmInstruction::RemoveLiquidity {
            amount: self.to_token_amount_option(ui_amount, &lp_token)?,
        }
        .to_vec()?;

        let accounts = match pool.route {
            PoolRoute::Raydium { .. } => {
                self.get_remove_liquidity_accounts_raydium(wallet_address, pool_name)?
            }
            PoolRoute::Saber { .. } => {
                self.get_remove_liquidity_accounts_saber(wallet_address, pool_name)?
            }
            PoolRoute::Orca { .. } => {
                self.get_remove_liquidity_accounts_orca(wallet_address, pool_name)?
            }
        };

        Ok(Instruction {
            program_id: pool.router_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for tokens swap
    pub fn new_instruction_swap(
        &self,
        wallet_address: &Pubkey,
        pool_code: &str,
        from_token: &str,
        to_token: &str,
        ui_amount_in: f64,
        min_ui_amount_out: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get pool to swap in
        let pool = self.find_pools(pool_code, from_token, to_token)?[0];
        let reverse = FarmClient::pool_has_reverse_tokens(&pool.name, from_token)?;

        // get tokens info
        let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;

        // convert amounts if wrapped tokens are used
        let mut max_amount_in = if reverse {
            self.to_token_amount_option(ui_amount_in, &token_b)?
        } else {
            self.to_token_amount_option(ui_amount_in, &token_a)?
        };
        let mut min_amount_out = if reverse {
            self.to_token_amount_option(min_ui_amount_out, &token_a)?
        } else {
            self.to_token_amount_option(min_ui_amount_out, &token_b)?
        };
        if let PoolRoute::Saber {
            wrapped_token_a_ref,
            wrapped_token_b_ref,
            ..
        } = pool.route
        {
            if let Some(token_ref) = wrapped_token_a_ref {
                let underlying_decimals =
                    token_a.ok_or(ProgramError::UninitializedAccount)?.decimals;
                let wrapped_decimals = self.get_token_by_ref(&token_ref)?.decimals;
                if reverse {
                    min_amount_out = account::to_amount_with_new_decimals(
                        min_amount_out,
                        underlying_decimals,
                        wrapped_decimals,
                    )?;
                } else {
                    max_amount_in = account::to_amount_with_new_decimals(
                        max_amount_in,
                        underlying_decimals,
                        wrapped_decimals,
                    )?;
                }
            }
            if let Some(token_ref) = wrapped_token_b_ref {
                let underlying_decimals =
                    token_b.ok_or(ProgramError::UninitializedAccount)?.decimals;
                let wrapped_decimals = self.get_token_by_ref(&token_ref)?.decimals;
                if reverse {
                    max_amount_in = account::to_amount_with_new_decimals(
                        max_amount_in,
                        underlying_decimals,
                        wrapped_decimals,
                    )?;
                } else {
                    min_amount_out = account::to_amount_with_new_decimals(
                        min_amount_out,
                        underlying_decimals,
                        wrapped_decimals,
                    )?;
                }
            }
        }

        // fill in accounts and instruction data
        let data = if reverse {
            AmmInstruction::Swap {
                token_a_amount_in: 0,
                token_b_amount_in: max_amount_in,
                min_token_amount_out: min_amount_out,
            }
        } else {
            AmmInstruction::Swap {
                token_a_amount_in: max_amount_in,
                token_b_amount_in: 0,
                min_token_amount_out: min_amount_out,
            }
        }
        .to_vec()?;

        let accounts = match pool.route {
            PoolRoute::Raydium { .. } => {
                self.get_swap_accounts_raydium(wallet_address, &pool.name)?
            }
            PoolRoute::Saber { .. } => self.get_swap_accounts_saber(wallet_address, &pool.name)?,
            PoolRoute::Orca { .. } => self.get_swap_accounts_orca(wallet_address, &pool.name)?,
        };

        Ok(Instruction {
            program_id: pool.router_program_id,
            data,
            accounts,
        })
    }

    pub fn new_instruction_wrap_token(
        &self,
        wallet_address: &Pubkey,
        pool_name: &str,
        token_to_wrap: TokenSelector,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get pool info
        let pool = self.get_pool(pool_name)?;

        // get underlying token info
        let token = if token_to_wrap == TokenSelector::TokenA {
            self.get_token_by_ref_from_cache(&pool.token_a_ref)?
        } else {
            self.get_token_by_ref_from_cache(&pool.token_b_ref)?
        };

        // fill in instruction data
        let data = AmmInstruction::WrapToken {
            amount: self.to_token_amount_option(ui_amount, &token)?,
        }
        .to_vec()?;

        let accounts = match pool.route {
            PoolRoute::Saber { .. } => {
                self.get_wrap_token_accounts_saber(wallet_address, pool_name, token_to_wrap)?
            }
            _ => {
                panic!("WrapToken instruction is not supported for this route type");
            }
        };

        Ok(Instruction {
            program_id: pool.router_program_id,
            data,
            accounts,
        })
    }

    pub fn new_instruction_unwrap_token(
        &self,
        wallet_address: &Pubkey,
        pool_name: &str,
        token_to_unwrap: TokenSelector,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get pool info
        let pool = self.get_pool(pool_name)?;

        let (accounts, decimals) = match pool.route {
            PoolRoute::Saber {
                wrapped_token_a_ref,
                wrapped_token_b_ref,
                ..
            } => {
                let token = if token_to_unwrap == TokenSelector::TokenA {
                    self.get_token_by_ref_from_cache(&wrapped_token_a_ref)?
                } else {
                    self.get_token_by_ref_from_cache(&wrapped_token_b_ref)?
                };
                (
                    self.get_wrap_token_accounts_saber(wallet_address, pool_name, token_to_unwrap)?,
                    token.ok_or(ProgramError::UninitializedAccount)?.decimals,
                )
            }
            _ => {
                panic!("UnwrapToken instruction is not supported for this route type");
            }
        };

        Ok(Instruction {
            program_id: pool.router_program_id,
            data: AmmInstruction::UnwrapToken {
                amount: self.ui_amount_to_tokens_with_decimals(ui_amount, decimals),
            }
            .to_vec()?,
            accounts,
        })
    }
}
