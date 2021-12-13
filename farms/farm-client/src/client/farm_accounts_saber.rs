//! Solana Farm Client Saber Farms accounts builder

use {
    crate::error::FarmClientError,
    solana_farm_sdk::{farm::FarmRoute, id::zero},
    solana_sdk::{instruction::AccountMeta, program_error::ProgramError, pubkey::Pubkey},
    std::vec::Vec,
};

use super::FarmClient;

impl FarmClient {
    /// Returns instruction accounts for tokens staking in a Saber farm
    pub fn get_stake_accounts_saber(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<Vec<AccountMeta>, FarmClientError> {
        // get farm info
        let farm = self.get_farm(farm_name)?;

        // get tokens info
        let lp_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;

        // get user accounts info
        let user_lp_token_account = self.get_token_account(wallet_address, &lp_token);

        // fill in accounts
        let mut accounts = vec![];
        if let FarmRoute::Saber {
            quarry, rewarder, ..
        } = farm.route
        {
            let user_info_account = self
                .get_stake_account(wallet_address, farm_name)?
                .ok_or(ProgramError::UninitializedAccount)?;
            let user_vault_account = self
                .get_token_account(&user_info_account, &lp_token)
                .ok_or(ProgramError::UninitializedAccount)?;

            accounts.push(AccountMeta::new_readonly(*wallet_address, true));
            accounts.push(AccountMeta::new(
                user_lp_token_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
            accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
            accounts.push(AccountMeta::new(user_info_account, false));
            accounts.push(AccountMeta::new(user_vault_account, false));
            accounts.push(AccountMeta::new(quarry, false));
            accounts.push(AccountMeta::new_readonly(rewarder, false));
        }

        Ok(accounts)
    }

    /// Returns instruction accounts for unstaking tokens from a Saber farm
    pub fn get_unstake_accounts_saber(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<Vec<AccountMeta>, FarmClientError> {
        // get farm info
        let farm = self.get_farm(farm_name)?;

        // get tokens info
        let lp_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;

        // get user accounts info
        let user_lp_token_account = self.get_token_account(wallet_address, &lp_token);

        // fill in accounts
        let mut accounts = vec![];
        if let FarmRoute::Saber {
            quarry, rewarder, ..
        } = farm.route
        {
            let user_info_account = self
                .get_stake_account(wallet_address, farm_name)?
                .ok_or(ProgramError::UninitializedAccount)?;
            let user_vault_account = self
                .get_token_account(&user_info_account, &lp_token)
                .ok_or(ProgramError::UninitializedAccount)?;

            accounts.push(AccountMeta::new_readonly(*wallet_address, true));
            accounts.push(AccountMeta::new(
                user_lp_token_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
            accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
            accounts.push(AccountMeta::new(user_info_account, false));
            accounts.push(AccountMeta::new(user_vault_account, false));
            accounts.push(AccountMeta::new(quarry, false));
            accounts.push(AccountMeta::new_readonly(rewarder, false));
        }

        Ok(accounts)
    }

    /// Returns instruction accounts for rewards harvesting in a Saber farm
    pub fn get_harvest_accounts_saber(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<Vec<AccountMeta>, FarmClientError> {
        // get farm info
        let farm = self.get_farm(farm_name)?;

        // get tokens info
        let sbr_token = self.get_token_by_ref_from_cache(&farm.reward_token_a_ref)?;
        let iou_token = self.get_token_by_ref_from_cache(&farm.reward_token_b_ref)?;

        // get user accounts info
        let user_sbr_token_account = self.get_token_account(wallet_address, &sbr_token);
        let user_iou_token_account = self.get_token_account(wallet_address, &iou_token);

        // fill in accounts
        let mut accounts = vec![];
        if let FarmRoute::Saber {
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
        } = farm.route
        {
            let user_info_account = self
                .get_stake_account(wallet_address, farm_name)?
                .ok_or(ProgramError::UninitializedAccount)?;

            accounts.push(AccountMeta::new_readonly(*wallet_address, true));
            accounts.push(AccountMeta::new(
                user_iou_token_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new(
                user_sbr_token_account.ok_or(ProgramError::UninitializedAccount)?,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(farm.farm_program_id, false));
            accounts.push(AccountMeta::new_readonly(spl_token::id(), false));
            accounts.push(AccountMeta::new(zero::id(), false));
            accounts.push(AccountMeta::new(user_info_account, false));

            accounts.push(AccountMeta::new_readonly(rewarder, false));
            accounts.push(AccountMeta::new_readonly(redeemer, false));
            accounts.push(AccountMeta::new_readonly(redeemer_program, false));
            accounts.push(AccountMeta::new(minter, false));
            accounts.push(AccountMeta::new(mint_wrapper, false));
            accounts.push(AccountMeta::new_readonly(mint_wrapper_program, false));
            accounts.push(AccountMeta::new(
                sbr_token.ok_or(ProgramError::UninitializedAccount)?.mint,
                false,
            ));
            accounts.push(AccountMeta::new(
                iou_token.ok_or(ProgramError::UninitializedAccount)?.mint,
                false,
            ));
            accounts.push(AccountMeta::new(iou_fees_account, false));
            accounts.push(AccountMeta::new(quarry, false));
            accounts.push(AccountMeta::new(sbr_vault, false));
            accounts.push(AccountMeta::new_readonly(mint_proxy_program, false));
            accounts.push(AccountMeta::new_readonly(mint_proxy_authority, false));
            accounts.push(AccountMeta::new_readonly(mint_proxy_state, false));
            accounts.push(AccountMeta::new(minter_info, false));
        }

        Ok(accounts)
    }
}
