//! Solana Farm Client System Instructions

use {
    crate::error::FarmClientError,
    solana_sdk::{instruction::Instruction, pubkey::Pubkey, system_instruction},
    spl_associated_token_account::create_associated_token_account,
    spl_token::instruction as spl_token_instruction,
};

use super::FarmClient;

impl FarmClient {
    /// Returns a new Instruction for creating system account
    pub fn new_instruction_create_system_account(
        &self,
        wallet_address: &Pubkey,
        new_account_address: &Pubkey,
        lamports: u64,
        space: usize,
        owner: &Pubkey,
    ) -> Result<Instruction, FarmClientError> {
        let lamports = if lamports == 0 {
            self.rpc_client
                .get_minimum_balance_for_rent_exemption(space)?
        } else {
            lamports
        };
        Ok(system_instruction::create_account(
            wallet_address,
            new_account_address,
            lamports,
            space as u64,
            owner,
        ))
    }

    /// Returns a new Instruction for closing system account
    pub fn new_instruction_close_system_account(
        &self,
        wallet_address: &Pubkey,
        target_account_address: &Pubkey,
    ) -> Result<Instruction, FarmClientError> {
        self.new_instruction_transfer(
            target_account_address,
            wallet_address,
            self.get_account_balance(wallet_address)?,
        )
    }

    /// Returns a new Instruction for creating system account with seed
    pub fn new_instruction_create_system_account_with_seed(
        &self,
        wallet_address: &Pubkey,
        base_address: &Pubkey,
        seed: &str,
        lamports: u64,
        space: usize,
        owner: &Pubkey,
    ) -> Result<Instruction, FarmClientError> {
        let lamports = if lamports == 0 {
            self.rpc_client
                .get_minimum_balance_for_rent_exemption(space)?
        } else {
            lamports
        };
        let to_pubkey = Pubkey::create_with_seed(base_address, seed, owner)?;
        Ok(system_instruction::create_account_with_seed(
            wallet_address,
            &to_pubkey,
            base_address,
            seed,
            lamports,
            space as u64,
            owner,
        ))
    }

    /// Returns a new Instruction for assigning system account to a program
    pub fn new_instruction_assign_system_account(
        &self,
        wallet_address: &Pubkey,
        program_address: &Pubkey,
    ) -> Result<Instruction, FarmClientError> {
        Ok(system_instruction::assign(wallet_address, program_address))
    }

    /// Creates the native SOL transfer instruction
    pub fn new_instruction_transfer(
        &self,
        wallet_address: &Pubkey,
        destination_wallet: &Pubkey,
        sol_ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        Ok(system_instruction::transfer(
            wallet_address,
            destination_wallet,
            self.ui_amount_to_tokens_with_decimals(sol_ui_amount, spl_token::native_mint::DECIMALS),
        ))
    }

    /// Creates a tokens transfer instruction
    pub fn new_instruction_token_transfer(
        &self,
        wallet_address: &Pubkey,
        token_name: &str,
        destination_wallet: &Pubkey,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        let token_addr = self.get_associated_token_address(wallet_address, token_name)?;
        let destination_address =
            self.get_associated_token_address(destination_wallet, token_name)?;
        Ok(spl_token_instruction::transfer(
            &spl_token::id(),
            &token_addr,
            &destination_address,
            wallet_address,
            &[],
            self.ui_amount_to_tokens(ui_amount, token_name)?,
        )?)
    }

    /// Creates a new Instruction for syncing token balance for the specified account
    pub fn new_instruction_sync_token_balance(
        &self,
        wallet_address: &Pubkey,
        token_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        let token_addr = self.get_associated_token_address(wallet_address, token_name)?;
        Ok(spl_token_instruction::sync_native(
            &spl_token::id(),
            &token_addr,
        )?)
    }

    /// Returns a new Instruction for creating associated token account
    pub fn new_instruction_create_token_account(
        &self,
        wallet_address: &Pubkey,
        token_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        let token = self.get_token(token_name)?;
        Ok(create_associated_token_account(
            wallet_address,
            wallet_address,
            &token.mint,
        ))
    }

    /// Returns a new Instruction for closing associated token account
    pub fn new_instruction_close_token_account(
        &self,
        wallet_address: &Pubkey,
        token_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        let token_addr = self.get_associated_token_address(wallet_address, token_name)?;
        Ok(spl_token_instruction::close_account(
            &spl_token::id(),
            &token_addr,
            wallet_address,
            wallet_address,
            &[],
        )?)
    }
}
