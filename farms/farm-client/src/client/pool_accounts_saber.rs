//! Solana Farm Client Saber Pools accounts builder

use {
    crate::error::FarmClientError,
    solana_farm_sdk::{pool::PoolRoute, token::TokenSelector},
    solana_sdk::{instruction::AccountMeta, program_error::ProgramError, pubkey::Pubkey, sysvar},
    std::vec::Vec,
};

use super::FarmClient;

impl FarmClient {
    /// Returns instruction accounts for adding liquidity to a Saber pool
    pub fn get_add_liquidity_accounts_saber(
        &self,
        wallet_address: &Pubkey,
        pool_name: &str,
    ) -> Result<Vec<AccountMeta>, FarmClientError> {
        // get pool info
        let pool = self.get_pool(pool_name)?;

        let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;
        let user_lp_token_account = self.get_token_account(wallet_address, &lp_token);

        // fill in accounts data
        let mut accounts = vec![];
        if let PoolRoute::Saber {
            swap_account,
            swap_authority,
            wrapped_token_a_ref,
            wrapped_token_b_ref,
            ..
        } = pool.route
        {
            let wrapped_token_a = self.get_token_by_ref_from_cache(&wrapped_token_a_ref)?;
            let user_token_a_account = if wrapped_token_a.is_some() {
                self.get_token_account(wallet_address, &wrapped_token_a)
            } else {
                let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
                self.get_token_account(wallet_address, &token_a)
            };
            let wrapped_token_b = self.get_token_by_ref_from_cache(&wrapped_token_b_ref)?;
            let user_token_b_account = if wrapped_token_b.is_some() {
                self.get_token_account(wallet_address, &wrapped_token_b)
            } else {
                let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;
                self.get_token_account(wallet_address, &token_b)
            };

            accounts.push(AccountMeta::new_readonly(*wallet_address, true));
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
            accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
            accounts.push(AccountMeta::new_readonly(sysvar::clock::id(), false));
            accounts.push(AccountMeta::new_readonly(swap_account, false));
            accounts.push(AccountMeta::new_readonly(swap_authority, false));
        }

        Ok(accounts)
    }

    /// Returns instruction accounts for removing liquidity from a Saber pool
    pub fn get_remove_liquidity_accounts_saber(
        &self,
        wallet_address: &Pubkey,
        pool_name: &str,
    ) -> Result<Vec<AccountMeta>, FarmClientError> {
        // get pool info
        let pool = self.get_pool(pool_name)?;

        let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;
        let user_lp_token_account = self.get_token_account(wallet_address, &lp_token);

        // fill in accounts data
        let mut accounts = vec![];
        if let PoolRoute::Saber {
            swap_account,
            swap_authority,
            fees_account_a,
            fees_account_b,
            wrapped_token_a_ref,
            wrapped_token_b_ref,
            ..
        } = pool.route
        {
            let wrapped_token_a = self.get_token_by_ref_from_cache(&wrapped_token_a_ref)?;
            let user_token_a_account = if wrapped_token_a.is_some() {
                self.get_token_account(wallet_address, &wrapped_token_a)
            } else {
                let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
                self.get_token_account(wallet_address, &token_a)
            };
            let wrapped_token_b = self.get_token_by_ref_from_cache(&wrapped_token_b_ref)?;
            let user_token_b_account = if wrapped_token_b.is_some() {
                self.get_token_account(wallet_address, &wrapped_token_b)
            } else {
                let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;
                self.get_token_account(wallet_address, &token_b)
            };

            accounts.push(AccountMeta::new_readonly(*wallet_address, true));
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
            accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
            accounts.push(AccountMeta::new_readonly(swap_account, false));
            accounts.push(AccountMeta::new_readonly(swap_authority, false));
            accounts.push(AccountMeta::new(fees_account_a, false));
            accounts.push(AccountMeta::new(fees_account_b, false));
        }

        Ok(accounts)
    }

    /// Returns instruction accounts for swapping tokens in a Saber pool
    pub fn get_swap_accounts_saber(
        &self,
        wallet_address: &Pubkey,
        pool_name: &str,
    ) -> Result<Vec<AccountMeta>, FarmClientError> {
        // get pool info
        let pool = self.get_pool(pool_name)?;

        // fill in accounts data
        let mut accounts = vec![];
        if let PoolRoute::Saber {
            swap_account,
            swap_authority,
            fees_account_a,
            fees_account_b,
            wrapped_token_a_ref,
            wrapped_token_b_ref,
            ..
        } = pool.route
        {
            let wrapped_token_a = self.get_token_by_ref_from_cache(&wrapped_token_a_ref)?;
            let user_token_a_account = if wrapped_token_a.is_some() {
                self.get_token_account(wallet_address, &wrapped_token_a)
            } else {
                let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
                self.get_token_account(wallet_address, &token_a)
            };
            let wrapped_token_b = self.get_token_by_ref_from_cache(&wrapped_token_b_ref)?;
            let user_token_b_account = if wrapped_token_b.is_some() {
                self.get_token_account(wallet_address, &wrapped_token_b)
            } else {
                let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;
                self.get_token_account(wallet_address, &token_b)
            };

            accounts.push(AccountMeta::new_readonly(*wallet_address, true));
            accounts.push(AccountMeta::new(
                user_token_a_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new(
                user_token_b_account.ok_or(ProgramError::UninitializedAccount)?,
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
            accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
            accounts.push(AccountMeta::new_readonly(sysvar::clock::id(), false));
            accounts.push(AccountMeta::new_readonly(swap_account, false));
            accounts.push(AccountMeta::new_readonly(swap_authority, false));
            accounts.push(AccountMeta::new(fees_account_a, false));
            accounts.push(AccountMeta::new(fees_account_b, false));
        }

        Ok(accounts)
    }

    /// Returns instruction accounts for wrapping token into a Saber decimal token
    pub fn get_wrap_token_accounts_saber(
        &self,
        wallet_address: &Pubkey,
        pool_name: &str,
        token_to_wrap: TokenSelector,
    ) -> Result<Vec<AccountMeta>, FarmClientError> {
        // get pool info
        let pool = self.get_pool(pool_name)?;

        // get underlying token info
        let token = if token_to_wrap == TokenSelector::TokenA {
            self.get_token_by_ref_from_cache(&pool.token_a_ref)?
        } else {
            self.get_token_by_ref_from_cache(&pool.token_b_ref)?
        };

        // get user accounts info
        let user_underlying_token_account = self.get_token_account(wallet_address, &token);

        // fill in accounts data
        let mut accounts = vec![];
        if let PoolRoute::Saber {
            swap_account: _,
            swap_authority: _,
            fees_account_a: _,
            fees_account_b: _,
            decimal_wrapper_program,
            wrapped_token_a_ref,
            wrapped_token_a_vault,
            decimal_wrapper_token_a,
            wrapped_token_b_ref,
            wrapped_token_b_vault,
            decimal_wrapper_token_b,
        } = pool.route
        {
            let (user_wrapped_token_account, wrapped_token, wrapped_token_vault, decimal_wrapper) =
                if token_to_wrap == TokenSelector::TokenA {
                    let wrapped_token_a = self.get_token_by_ref_from_cache(&wrapped_token_a_ref)?;
                    (
                        self.get_token_account(wallet_address, &wrapped_token_a),
                        wrapped_token_a,
                        wrapped_token_a_vault,
                        decimal_wrapper_token_a,
                    )
                } else {
                    let wrapped_token_b = self.get_token_by_ref_from_cache(&wrapped_token_b_ref)?;
                    (
                        self.get_token_account(wallet_address, &wrapped_token_b),
                        wrapped_token_b,
                        wrapped_token_b_vault,
                        decimal_wrapper_token_b,
                    )
                };

            accounts.push(AccountMeta::new_readonly(*wallet_address, true));
            accounts.push(AccountMeta::new(
                user_underlying_token_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(
                token.ok_or(ProgramError::UninitializedAccount)?.mint,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
            accounts.push(AccountMeta::new_readonly(decimal_wrapper_program, false));
            accounts.push(AccountMeta::new(
                user_wrapped_token_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new(
                wrapped_token
                    .ok_or(ProgramError::UninitializedAccount)?
                    .mint,
                false,
            ));
            accounts.push(AccountMeta::new(
                wrapped_token_vault.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(
                decimal_wrapper.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
        }

        Ok(accounts)
    }
}
