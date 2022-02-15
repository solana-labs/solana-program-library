//! Solana Farm Client Farm Instructions

use {
    crate::error::FarmClientError,
    solana_farm_sdk::{farm::FarmRoute, instruction::amm::AmmInstruction},
    solana_sdk::{instruction::Instruction, pubkey::Pubkey},
};

use super::FarmClient;

impl FarmClient {
    /// Creates a new Instruction for tokens staking
    pub fn new_instruction_stake(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get farm info
        let farm = self.get_farm(farm_name)?;
        let lp_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;

        // fill in accounts and instruction data
        let data = AmmInstruction::Stake {
            amount: self.to_token_amount_option(ui_amount, &lp_token)?,
        }
        .to_vec()?;

        let accounts = match farm.route {
            FarmRoute::Raydium { .. } => {
                self.get_stake_accounts_raydium(wallet_address, farm_name)?
            }
            FarmRoute::Saber { .. } => self.get_stake_accounts_saber(wallet_address, farm_name)?,
            FarmRoute::Orca { .. } => self.get_stake_accounts_orca(wallet_address, farm_name)?,
        };

        Ok(Instruction {
            program_id: farm.router_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for tokens unstaking
    pub fn new_instruction_unstake(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        // get farm info
        let farm = self.get_farm(farm_name)?;
        let lp_token = self.get_token_by_ref_from_cache(&farm.lp_token_ref)?;

        // fill in accounts and instruction data
        let data = AmmInstruction::Unstake {
            amount: self.to_token_amount_option(ui_amount, &lp_token)?,
        }
        .to_vec()?;

        let accounts = match farm.route {
            FarmRoute::Raydium { .. } => {
                self.get_unstake_accounts_raydium(wallet_address, farm_name)?
            }
            FarmRoute::Saber { .. } => {
                self.get_unstake_accounts_saber(wallet_address, farm_name)?
            }
            FarmRoute::Orca { .. } => self.get_unstake_accounts_orca(wallet_address, farm_name)?,
        };

        Ok(Instruction {
            program_id: farm.router_program_id,
            data,
            accounts,
        })
    }

    /// Creates a new Instruction for rewards harvesting
    pub fn new_instruction_harvest(
        &self,
        wallet_address: &Pubkey,
        farm_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // get farm info
        let farm = self.get_farm(farm_name)?;

        // fill in accounts and instruction data
        let data = AmmInstruction::Harvest.to_vec()?;

        let accounts = match farm.route {
            FarmRoute::Raydium { .. } => {
                self.get_harvest_accounts_raydium(wallet_address, farm_name)?
            }
            FarmRoute::Saber { .. } => {
                self.get_harvest_accounts_saber(wallet_address, farm_name)?
            }
            FarmRoute::Orca { .. } => self.get_harvest_accounts_orca(wallet_address, farm_name)?,
        };

        Ok(Instruction {
            program_id: farm.router_program_id,
            data,
            accounts,
        })
    }
}
