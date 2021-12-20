//! Solana Farm Client Raydium Farms accounts builder

use {
    crate::error::FarmClientError,
    solana_farm_sdk::{farm::FarmRoute, id::zero},
    solana_sdk::{instruction::AccountMeta, program_error::ProgramError, pubkey::Pubkey, sysvar},
    std::vec::Vec,
};

use super::FarmClient;

impl FarmClient {
    /// Returns instruction accounts for tokens staking in a Raydium farm
    pub fn get_stake_accounts_raydium(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<Vec<AccountMeta>, FarmClientError> {
        // get farm info
        let farm = self.get_farm(farm_name)?;

        // get tokens info
        let token_a = self.get_token_by_ref_from_cache(&farm.reward_token_a_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&farm.reward_token_b_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;

        // get user accounts info
        let user_reward_token_a_account = self.get_token_account(wallet_address, &token_a);
        let user_reward_token_b_account = self.get_token_account(wallet_address, &token_b);
        let user_lp_token_account = self.get_token_account(wallet_address, &lp_token);

        // fill in accounts
        let mut accounts = vec![];
        if let FarmRoute::Raydium {
            farm_id,
            farm_authority,
            farm_lp_token_account,
            farm_reward_token_a_account,
            farm_reward_token_b_account,
        } = farm.route
        {
            let user_info_account = self.get_stake_account(wallet_address, farm_name)?;

            accounts.push(AccountMeta::new_readonly(*wallet_address, true));
            accounts.push(AccountMeta::new(
                user_info_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new(
                user_lp_token_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new(
                user_reward_token_a_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new(
                user_reward_token_b_account
                    .or_else(|| Some(zero::id()))
                    .unwrap(),
                false,
            ));
            accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
            accounts.push(AccountMeta::new(farm_lp_token_account, false));
            accounts.push(AccountMeta::new(farm_reward_token_a_account, false));
            accounts.push(AccountMeta::new(
                farm_reward_token_b_account
                    .or_else(|| Some(zero::id()))
                    .unwrap(),
                false,
            ));
            accounts.push(AccountMeta::new_readonly(sysvar::clock::id(), false));
            accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
            accounts.push(AccountMeta::new(farm_id, false));
            accounts.push(AccountMeta::new_readonly(farm_authority, false));
        }

        Ok(accounts)
    }

    /// Returns instruction accounts for unstaking tokens from a Raydium farm
    pub fn get_unstake_accounts_raydium(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<Vec<AccountMeta>, FarmClientError> {
        // get farm info
        let farm = self.get_farm(farm_name)?;

        // get tokens info
        let token_a = self.get_token_by_ref_from_cache(&farm.reward_token_a_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&farm.reward_token_b_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;

        // get user accounts info
        let user_reward_token_a_account = self.get_token_account(wallet_address, &token_a);
        let user_reward_token_b_account = self.get_token_account(wallet_address, &token_b);
        let user_lp_token_account = self.get_token_account(wallet_address, &lp_token);

        // fill in accounts
        let mut accounts = vec![];
        if let FarmRoute::Raydium {
            farm_id,
            farm_authority,
            farm_lp_token_account,
            farm_reward_token_a_account,
            farm_reward_token_b_account,
        } = farm.route
        {
            let user_info_account = self.get_stake_account(wallet_address, farm_name)?;

            accounts.push(AccountMeta::new_readonly(*wallet_address, true));
            accounts.push(AccountMeta::new(
                user_info_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new(
                user_lp_token_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new(
                user_reward_token_a_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new(
                user_reward_token_b_account
                    .or_else(|| Some(zero::id()))
                    .unwrap(),
                false,
            ));
            accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
            accounts.push(AccountMeta::new(farm_lp_token_account, false));
            accounts.push(AccountMeta::new(farm_reward_token_a_account, false));
            accounts.push(AccountMeta::new(
                farm_reward_token_b_account
                    .or_else(|| Some(zero::id()))
                    .unwrap(),
                false,
            ));
            accounts.push(AccountMeta::new_readonly(sysvar::clock::id(), false));
            accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
            accounts.push(AccountMeta::new(farm_id, false));
            accounts.push(AccountMeta::new_readonly(farm_authority, false));
        }

        Ok(accounts)
    }

    /// Returns instruction accounts for rewards harvesting in a Raydium farm
    pub fn get_harvest_accounts_raydium(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<Vec<AccountMeta>, FarmClientError> {
        self.get_stake_accounts_raydium(wallet_address, farm_name)
    }
}
