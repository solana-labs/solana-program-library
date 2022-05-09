//! Solana Farm Client System Instructions

use {
    crate::error::FarmClientError,
    solana_account_decoder::parse_token::{parse_token, TokenAccountType},
    solana_sdk::{instruction::Instruction, pubkey::Pubkey, system_instruction, system_program},
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
        if wallet_address == destination_wallet {
            return Err(FarmClientError::ValueError(
                "Source and destination addresses are the same".to_string(),
            ));
        }
        if let Ok(account) = self.rpc_client.get_account(destination_wallet) {
            if destination_wallet != &self.get_associated_token_address(wallet_address, "SOL")?
                && (account.owner != system_program::id() || !account.data.is_empty())
            {
                return Err(FarmClientError::ValueError(
                    "Destination account is not a SOL wallet".to_string(),
                ));
            }
        }
        Ok(system_instruction::transfer(
            wallet_address,
            destination_wallet,
            self.ui_amount_to_tokens_with_decimals(
                sol_ui_amount,
                spl_token::native_mint::DECIMALS,
            )?,
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
        if let Ok(account) = self.rpc_client.get_account(destination_wallet) {
            if account.owner != system_program::id() || !account.data.is_empty() {
                return Err(FarmClientError::ValueError(
                    "Destination account is not a SOL wallet".to_string(),
                ));
            }
        }
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

    /// Creates a new complete set of instructions for SOL wrapping
    pub fn all_instructions_wrap_sol(
        &self,
        wallet_address: &Pubkey,
        ui_amount: f64,
    ) -> Result<Vec<Instruction>, FarmClientError> {
        let target_account = self.get_associated_token_address(wallet_address, "SOL")?;
        let mut inst = vec![];
        if !self.has_active_token_account(wallet_address, "SOL") {
            inst.push(self.new_instruction_create_token_account(wallet_address, "SOL")?);
        } else {
            self.check_ata_owner(wallet_address, "SOL")?;
        }
        inst.push(self.new_instruction_transfer(wallet_address, &target_account, ui_amount)?);
        Ok(inst)
    }

    /// Creates a new complete set of instructions for SOL unwrapping
    pub fn all_instructions_unwrap_sol(
        &self,
        wallet_address: &Pubkey,
    ) -> Result<Vec<Instruction>, FarmClientError> {
        let inst = vec![self.new_instruction_close_token_account(wallet_address, "SOL")?];
        Ok(inst)
    }

    /// Creates a new complete set of instructions for tokens transfer
    pub fn all_instructions_token_transfer(
        &self,
        wallet_address: &Pubkey,
        token_name: &str,
        destination_wallet: &Pubkey,
        ui_amount: f64,
    ) -> Result<Vec<Instruction>, FarmClientError> {
        if wallet_address == destination_wallet {
            return Err(FarmClientError::ValueError(
                "Source and destination addresses are the same".to_string(),
            ));
        }
        let mut inst = vec![];
        if !self.has_active_token_account(wallet_address, token_name) {
            return Err(FarmClientError::RecordNotFound(format!(
                "Source account with token {}",
                token_name
            )));
        }
        let data = self.rpc_client.get_account_data(destination_wallet)?;
        let res = parse_token(data.as_slice(), Some(0));
        if let Ok(TokenAccountType::Account(_)) = res {
            return Err(FarmClientError::ValueError(
                "Destination must be a base wallet address, token address will be derived"
                    .to_string(),
            ));
        }

        if !self.has_active_token_account(destination_wallet, token_name) {
            let token = self.get_token(token_name)?;
            inst.push(create_associated_token_account(
                wallet_address,
                destination_wallet,
                &token.mint,
            ));
        } else {
            self.check_ata_owner(destination_wallet, token_name)?;
        }

        inst.push(self.new_instruction_token_transfer(
            wallet_address,
            token_name,
            destination_wallet,
            ui_amount,
        )?);

        Ok(inst)
    }
}
