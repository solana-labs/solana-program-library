//! Solana Farm Client Raydium Farms accounts builder

use {
    crate::error::FarmClientError,
    solana_farm_sdk::{farm::FarmRoute, id::zero},
    solana_sdk::{
        instruction::AccountMeta, program_error::ProgramError, pubkey::Pubkey, system_program,
        sysvar,
    },
    std::vec::Vec,
};

use super::FarmClient;

impl FarmClient {
    /// Returns instruction accounts for initializing a new User in a Raydium farm
    pub fn get_user_init_accounts_raydium(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<Vec<AccountMeta>, FarmClientError> {
        // get farm info
        let farm = self.get_farm(farm_name)?;
        let farm_metadata = self.get_farm_ref(farm_name)?;

        let farm_id = match farm.route {
            FarmRoute::Raydium { farm_id, .. } => farm_id,
            _ => unreachable!(),
        };

        let farmer = Pubkey::find_program_address(
            &[b"Miner", &farm_id.to_bytes(), &wallet_address.to_bytes()],
            &farm.router_program_id,
        )
        .0;

        let accounts = vec![
            AccountMeta::new(*wallet_address, true),
            AccountMeta::new_readonly(*wallet_address, false),
            AccountMeta::new(farmer, false),
            AccountMeta::new_readonly(farm_metadata, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        Ok(accounts)
    }

    /// Returns instruction accounts for tokens staking in a Raydium farm
    pub fn get_stake_accounts_raydium(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<Vec<AccountMeta>, FarmClientError> {
        // get farm info
        let farm = self.get_farm(farm_name)?;

        // get tokens info
        let token_a = self.get_token_by_ref_from_cache(&farm.first_reward_token_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&farm.second_reward_token_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;

        // get user accounts info
        let user_first_reward_token_account = self.get_token_account(wallet_address, &token_a);
        let user_second_reward_token_account = self.get_token_account(wallet_address, &token_b);
        let user_lp_token_account = self.get_token_account(wallet_address, &lp_token);

        // fill in accounts
        let mut accounts = vec![];
        if let FarmRoute::Raydium {
            farm_id,
            farm_authority,
            farm_lp_token_account,
            farm_first_reward_token_account,
            farm_second_reward_token_account,
        } = farm.route
        {
            let user_info_account = self.get_stake_account(wallet_address, farm_name)?;

            accounts.push(AccountMeta::new_readonly(*wallet_address, true));
            accounts.push(AccountMeta::new(user_info_account, false));
            accounts.push(AccountMeta::new(
                user_lp_token_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new(
                user_first_reward_token_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new(
                user_second_reward_token_account
                    .or_else(|| Some(zero::id()))
                    .unwrap(),
                false,
            ));
            accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
            accounts.push(AccountMeta::new(farm_lp_token_account, false));
            accounts.push(AccountMeta::new(farm_first_reward_token_account, false));
            accounts.push(AccountMeta::new(
                farm_second_reward_token_account
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
        let token_a = self.get_token_by_ref_from_cache(&farm.first_reward_token_ref)?;
        let token_b = self.get_token_by_ref_from_cache(&farm.second_reward_token_ref)?;
        let lp_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;

        // get user accounts info
        let user_first_reward_token_account = self.get_token_account(wallet_address, &token_a);
        let user_second_reward_token_account = self.get_token_account(wallet_address, &token_b);
        let user_lp_token_account = self.get_token_account(wallet_address, &lp_token);

        // fill in accounts
        let mut accounts = vec![];
        if let FarmRoute::Raydium {
            farm_id,
            farm_authority,
            farm_lp_token_account,
            farm_first_reward_token_account,
            farm_second_reward_token_account,
        } = farm.route
        {
            let user_info_account = self.get_stake_account(wallet_address, farm_name)?;

            accounts.push(AccountMeta::new_readonly(*wallet_address, true));
            accounts.push(AccountMeta::new(user_info_account, false));
            accounts.push(AccountMeta::new(
                user_lp_token_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new(
                user_first_reward_token_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new(
                user_second_reward_token_account
                    .or_else(|| Some(zero::id()))
                    .unwrap(),
                false,
            ));
            accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
            accounts.push(AccountMeta::new(farm_lp_token_account, false));
            accounts.push(AccountMeta::new(farm_first_reward_token_account, false));
            accounts.push(AccountMeta::new(
                farm_second_reward_token_account
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
