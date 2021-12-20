//! Solana Farm Client Orca Pools accounts builder

use {
    crate::error::FarmClientError,
    solana_farm_sdk::pool::PoolRoute,
    solana_sdk::{instruction::AccountMeta, program_error::ProgramError, pubkey::Pubkey},
    std::vec::Vec,
};

use super::FarmClient;

impl FarmClient {
    /// Returns instruction accounts for adding liquidity to an Orca pool
    pub fn get_add_liquidity_accounts_orca(
        &self,
        wallet_address: &Pubkey,
        pool_name: &str,
    ) -> Result<Vec<AccountMeta>, FarmClientError> {
        // get pool info
        let pool = self.get_pool(pool_name)?;

        // get tokens info
        let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;

        // get user accounts info
        let user_token_a_account = self.get_token_account(wallet_address, &token_a);
        let user_token_b_account = self.get_token_account(wallet_address, &token_b);
        let user_lp_token_account = self.get_token_account(wallet_address, &lp_token);

        // fill in accounts data
        let mut accounts = vec![];
        if let PoolRoute::Orca {
            amm_id,
            amm_authority,
            ..
        } = pool.route
        {
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
            accounts.push(AccountMeta::new_readonly(amm_id, false));
            accounts.push(AccountMeta::new_readonly(amm_authority, false));
        }

        Ok(accounts)
    }

    /// Returns instruction accounts for removing liquidity from an Orca pool
    pub fn get_remove_liquidity_accounts_orca(
        &self,
        wallet_address: &Pubkey,
        pool_name: &str,
    ) -> Result<Vec<AccountMeta>, FarmClientError> {
        // get pool info
        let pool = self.get_pool(pool_name)?;

        // get tokens info
        let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;

        // get user accounts info
        let user_token_a_account = self.get_token_account(wallet_address, &token_a);
        let user_token_b_account = self.get_token_account(wallet_address, &token_b);
        let user_lp_token_account = self.get_token_account(wallet_address, &lp_token);

        // fill in accounts data
        let mut accounts = vec![];
        if let PoolRoute::Orca {
            amm_id,
            amm_authority,
            fees_account,
        } = pool.route
        {
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
            accounts.push(AccountMeta::new_readonly(amm_id, false));
            accounts.push(AccountMeta::new_readonly(amm_authority, false));
            accounts.push(AccountMeta::new(fees_account, false));
        }

        Ok(accounts)
    }

    /// Returns instruction accounts for swapping tokens in an Orca pool
    pub fn get_swap_accounts_orca(
        &self,
        wallet_address: &Pubkey,
        pool_name: &str,
    ) -> Result<Vec<AccountMeta>, FarmClientError> {
        // get pool info
        let pool = self.get_pool(pool_name)?;

        // get tokens info
        let token_a = self.get_token_by_ref_from_cache(&pool.token_a_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&pool.token_b_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&pool.lp_token_ref)?;

        // get user accounts info
        let user_token_a_account = self.get_token_account(wallet_address, &token_a);
        let user_token_b_account = self.get_token_account(wallet_address, &token_b);

        // fill in accounts data
        let mut accounts = vec![];
        if let PoolRoute::Orca {
            amm_id,
            amm_authority,
            fees_account,
        } = pool.route
        {
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
            accounts.push(AccountMeta::new(
                lp_token.ok_or(ProgramError::UninitializedAccount)?.mint,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
            accounts.push(AccountMeta::new_readonly(amm_id, false));
            accounts.push(AccountMeta::new_readonly(amm_authority, false));
            accounts.push(AccountMeta::new(fees_account, false));
        }

        Ok(accounts)
    }
}
