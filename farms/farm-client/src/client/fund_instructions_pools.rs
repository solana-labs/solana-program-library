//! Solana Farm Client Fund Instructions for pools, farms, and vaults

use {
    crate::error::FarmClientError,
    solana_farm_sdk::{
        fund::FundVaultType,
        instruction::{amm::AmmInstruction, fund::FundInstruction, vault::VaultInstruction},
        Protocol,
    },
    solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        sysvar,
    },
};

use super::FarmClient;

impl FarmClient {
    /// Creates a new Instruction for initializing a new user for the Farm in the Fund
    pub fn new_instruction_fund_user_init_farm(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        farm_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let farm_inst = self.new_instruction_user_init(&fund.fund_authority, farm_name)?;
        self.build_fund_instruction(
            admin_address,
            fund_name,
            &farm_inst,
            farm_name,
            FundVaultType::Farm,
        )
    }

    /// Creates a new Instruction for adding liquidity to the Pool in the Fund
    pub fn new_instruction_fund_add_liquidity_pool(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        pool_name: &str,
        max_token_a_ui_amount: f64,
        max_token_b_ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let pool_inst = self.new_instruction_add_liquidity_pool(
            &fund.fund_authority,
            pool_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )?;
        self.build_fund_instruction(
            admin_address,
            fund_name,
            &pool_inst,
            pool_name,
            FundVaultType::Pool,
        )
    }

    /// Creates a new Instruction for removing liquidity from the Pool in the Fund
    pub fn new_instruction_fund_remove_liquidity_pool(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        pool_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let pool_inst =
            self.new_instruction_remove_liquidity_pool(&fund.fund_authority, pool_name, ui_amount)?;
        self.build_fund_instruction(
            admin_address,
            fund_name,
            &pool_inst,
            pool_name,
            FundVaultType::Pool,
        )
    }

    /// Creates a new Instruction for tokens swap
    #[allow(clippy::too_many_arguments)]
    pub fn new_instruction_fund_swap(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        protocol: Protocol,
        from_token: &str,
        to_token: &str,
        ui_amount_in: f64,
        min_ui_amount_out: f64,
    ) -> Result<Instruction, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let pool = self.find_pools(protocol, from_token, to_token)?[0];
        let pool_inst = self.new_instruction_swap(
            &fund.fund_authority,
            protocol,
            from_token,
            to_token,
            ui_amount_in,
            min_ui_amount_out,
        )?;
        self.build_fund_instruction(
            admin_address,
            fund_name,
            &pool_inst,
            &pool.name,
            FundVaultType::Pool,
        )
    }

    /// Creates a new Instruction for tokens staking to the Farm in the Fund
    pub fn new_instruction_fund_stake(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        farm_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let farm_inst = self.new_instruction_stake(&fund.fund_authority, farm_name, ui_amount)?;
        self.build_fund_instruction(
            admin_address,
            fund_name,
            &farm_inst,
            farm_name,
            FundVaultType::Farm,
        )
    }

    /// Creates a new Instruction for tokens unstaking from the Farm in the Fund
    pub fn new_instruction_fund_unstake(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        farm_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let farm_inst = self.new_instruction_unstake(&fund.fund_authority, farm_name, ui_amount)?;
        self.build_fund_instruction(
            admin_address,
            fund_name,
            &farm_inst,
            farm_name,
            FundVaultType::Farm,
        )
    }

    /// Creates a new Instruction for rewards harvesting from the Farm in the Fund
    pub fn new_instruction_fund_harvest(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        farm_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let farm_inst = self.new_instruction_harvest(&fund.fund_authority, farm_name)?;
        self.build_fund_instruction(
            admin_address,
            fund_name,
            &farm_inst,
            farm_name,
            FundVaultType::Farm,
        )
    }

    /// Creates a new Instruction for initializing a new user for the Vault in the Fund
    pub fn new_instruction_fund_user_init_vault(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        vault_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let vault_inst = self.new_instruction_user_init_vault(&fund.fund_authority, vault_name)?;
        self.build_fund_vault_instruction(admin_address, fund_name, &vault_inst, vault_name)
    }

    /// Creates a new Instruction for adding liquidity to the Vault in the Fund
    pub fn new_instruction_fund_add_liquidity_vault(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        vault_name: &str,
        max_token_a_ui_amount: f64,
        max_token_b_ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let vault_inst = self.new_instruction_add_liquidity_vault(
            &fund.fund_authority,
            vault_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )?;
        self.build_fund_vault_instruction(admin_address, fund_name, &vault_inst, vault_name)
    }

    /// Creates a new Instruction for locking liquidity in the Vault in the Fund
    pub fn new_instruction_fund_lock_liquidity_vault(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let vault_inst =
            self.new_instruction_lock_liquidity_vault(&fund.fund_authority, vault_name, ui_amount)?;
        self.build_fund_vault_instruction(admin_address, fund_name, &vault_inst, vault_name)
    }

    /// Creates a new Instruction for unlocking liquidity in the Vault in the Fund
    pub fn new_instruction_fund_unlock_liquidity_vault(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let vault_inst = self.new_instruction_unlock_liquidity_vault(
            &fund.fund_authority,
            vault_name,
            ui_amount,
        )?;
        self.build_fund_vault_instruction(admin_address, fund_name, &vault_inst, vault_name)
    }

    /// Creates a new Instruction for removing liquidity from the Vault in the Fund
    pub fn new_instruction_fund_remove_liquidity_vault(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Instruction, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let vault_inst = self.new_instruction_remove_liquidity_vault(
            &fund.fund_authority,
            vault_name,
            ui_amount,
        )?;
        self.build_fund_vault_instruction(admin_address, fund_name, &vault_inst, vault_name)
    }

    /// Creates a new complete set of Instructions for adding liquidity to the Pool in the Fund
    pub fn all_instructions_fund_add_liquidity_pool(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        pool_name: &str,
        max_token_a_ui_amount: f64,
        max_token_b_ui_amount: f64,
    ) -> Result<Vec<Instruction>, FarmClientError> {
        if max_token_a_ui_amount < 0.0
            || max_token_b_ui_amount < 0.0
            || (max_token_a_ui_amount == 0.0 && max_token_b_ui_amount == 0.0)
        {
            return Err(FarmClientError::ValueError(format!(
                "Invalid add liquidity amounts {} and {} specified for Pool {}: Must be greater or equal to zero and at least one non-zero.",
                max_token_a_ui_amount, max_token_b_ui_amount, pool_name
            )));
        }
        // check custodies
        let mut inst = Vec::<Instruction>::new();
        self.check_fund_pool_custodies(
            admin_address,
            fund_name,
            pool_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
            0.0,
            true,
            &mut inst,
        )?;

        // create and send the instruction
        inst.push(self.new_instruction_fund_add_liquidity_pool(
            admin_address,
            fund_name,
            pool_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )?);

        Ok(inst)
    }

    /// Creates a new complete set of Instructions for removing liquidity from the Pool in the Fund
    pub fn all_instructions_fund_remove_liquidity_pool(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        pool_name: &str,
        ui_amount: f64,
    ) -> Result<Vec<Instruction>, FarmClientError> {
        // check custodies
        let mut inst = Vec::<Instruction>::new();
        self.check_fund_pool_custodies(
            admin_address,
            fund_name,
            pool_name,
            0.0,
            0.0,
            ui_amount,
            true,
            &mut inst,
        )?;

        // create and send the instruction
        inst.push(self.new_instruction_fund_remove_liquidity_pool(
            admin_address,
            fund_name,
            pool_name,
            ui_amount,
        )?);

        Ok(inst)
    }

    /// Creates a new complete set of Instructions for swapping tokens in the Fund
    #[allow(clippy::too_many_arguments)]
    pub fn all_instructions_fund_swap(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        protocol: Protocol,
        from_token: &str,
        to_token: &str,
        ui_amount_in: f64,
        min_ui_amount_out: f64,
    ) -> Result<Vec<Instruction>, FarmClientError> {
        // check amount
        if ui_amount_in < 0.0 {
            return Err(FarmClientError::ValueError(format!(
                "Invalid ui_amount_in {} specified for swap: Must be zero or greater.",
                ui_amount_in
            )));
        }
        // check custodies
        let mut inst = Vec::<Instruction>::new();
        self.check_fund_custody(
            admin_address,
            fund_name,
            from_token,
            ui_amount_in,
            &mut inst,
        )?;
        self.check_fund_custody(admin_address, fund_name, to_token, 0.0, &mut inst)?;

        // create and send the instruction
        inst.push(self.new_instruction_fund_swap(
            admin_address,
            fund_name,
            protocol,
            from_token,
            to_token,
            ui_amount_in,
            min_ui_amount_out,
        )?);

        Ok(inst)
    }

    /// Creates a new complete set of Instructions for staking tokens to the Farm in the Fund
    pub fn all_instructions_fund_stake(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        farm_name: &str,
        ui_amount: f64,
    ) -> Result<Vec<Instruction>, FarmClientError> {
        // check custodies
        let mut inst = Vec::<Instruction>::new();
        self.check_fund_farm_custodies(admin_address, fund_name, farm_name, ui_amount, &mut inst)?;

        // create and send the instruction
        inst.push(self.new_instruction_fund_stake(
            admin_address,
            fund_name,
            farm_name,
            ui_amount,
        )?);

        Ok(inst)
    }

    /// Creates a new complete set of Instructions for unstaking tokens from the Farm in the Fund
    pub fn all_instructions_fund_unstake(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        farm_name: &str,
        ui_amount: f64,
    ) -> Result<Vec<Instruction>, FarmClientError> {
        // check custodies
        let mut inst = Vec::<Instruction>::new();
        self.check_fund_farm_custodies(admin_address, fund_name, farm_name, 0.0, &mut inst)?;

        // create and send the instruction
        inst.push(self.new_instruction_fund_unstake(
            admin_address,
            fund_name,
            farm_name,
            ui_amount,
        )?);

        Ok(inst)
    }

    /// Creates a new complete set of Instructions for harvesting rewards from the Farm in the Fund
    pub fn all_instructions_fund_harvest(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        farm_name: &str,
    ) -> Result<Vec<Instruction>, FarmClientError> {
        // check custodies
        let mut inst = Vec::<Instruction>::new();
        self.check_fund_farm_custodies(admin_address, fund_name, farm_name, 0.0, &mut inst)?;

        // create and send the instruction
        inst.push(self.new_instruction_fund_harvest(admin_address, fund_name, farm_name)?);

        Ok(inst)
    }

    /// Creates a new complete set of Instructions for adding liquidity to the Vault in the Fund
    pub fn all_instructions_fund_add_liquidity_vault(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        vault_name: &str,
        max_token_a_ui_amount: f64,
        max_token_b_ui_amount: f64,
    ) -> Result<Vec<Instruction>, FarmClientError> {
        if max_token_a_ui_amount < 0.0
            || max_token_b_ui_amount < 0.0
            || (max_token_a_ui_amount == 0.0 && max_token_b_ui_amount == 0.0)
        {
            return Err(FarmClientError::ValueError(format!(
                "Invalid add liquidity amounts {} and {} specified for Vault {}: Must be greater or equal to zero and at least one non-zero.",
                max_token_a_ui_amount, max_token_b_ui_amount, vault_name
            )));
        }

        let mut inst = Vec::<Instruction>::new();
        self.check_fund_vault_custodies(
            admin_address,
            fund_name,
            vault_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
            0.0,
            true,
            true,
            &mut inst,
        )?;

        // insert add liquidity instruction
        inst.push(self.new_instruction_fund_add_liquidity_vault(
            admin_address,
            fund_name,
            vault_name,
            max_token_a_ui_amount,
            max_token_b_ui_amount,
        )?);

        // lock liquidity if required by the vault
        let vault = self.get_vault(vault_name)?;
        if vault.lock_required {
            let lock_inst = self.new_instruction_fund_lock_liquidity_vault(
                admin_address,
                fund_name,
                vault_name,
                0.0,
            )?;
            inst.push(lock_inst);
        }

        Ok(inst)
    }

    /// Creates a new complete set of Instructions for adding locked liquidity to the Vault in the Fund
    pub fn all_instructions_fund_add_locked_liquidity_vault(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Vec<Instruction>, FarmClientError> {
        // check fund accounts
        let mut inst = Vec::<Instruction>::new();
        self.check_fund_vault_custodies(
            admin_address,
            fund_name,
            vault_name,
            0.0,
            0.0,
            0.0,
            true,
            false,
            &mut inst,
        )?;

        // check if the user has locked balance
        if ui_amount > 0.0 {
            let fund = self.get_fund(fund_name)?;
            let lp_debt = self
                .get_vault_user_info(&fund.fund_authority, vault_name)?
                .lp_tokens_debt;
            let pool_token_decimals = self.get_vault_lp_token_decimals(vault_name)?;
            if self.tokens_to_ui_amount_with_decimals(lp_debt, pool_token_decimals) < ui_amount {
                return Err(FarmClientError::InsufficientBalance(
                    "Not enough locked tokens to deposit".to_string(),
                ));
            }
        }

        inst.push(self.new_instruction_fund_lock_liquidity_vault(
            admin_address,
            fund_name,
            vault_name,
            ui_amount,
        )?);

        Ok(inst)
    }

    /// Creates a new complete set of Instructions for removing liquidity from the Vault in the Fund
    pub fn all_instructions_fund_remove_liquidity_vault(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Vec<Instruction>, FarmClientError> {
        // check user accounts
        let vault = self.get_vault(vault_name)?;
        let mut inst = Vec::<Instruction>::new();
        self.check_fund_vault_custodies(
            admin_address,
            fund_name,
            vault_name,
            0.0,
            0.0,
            0.0,
            true,
            false,
            &mut inst,
        )?;

        // unlock liquidity first if required by the vault
        if vault.unlock_required {
            inst.push(self.new_instruction_fund_unlock_liquidity_vault(
                admin_address,
                fund_name,
                vault_name,
                ui_amount,
            )?);
            inst.push(self.new_instruction_fund_remove_liquidity_vault(
                admin_address,
                fund_name,
                vault_name,
                0.0,
            )?);
        } else {
            // remove liquidity
            inst.push(self.new_instruction_fund_remove_liquidity_vault(
                admin_address,
                fund_name,
                vault_name,
                ui_amount,
            )?);
        }

        Ok(inst)
    }

    /// Creates a new complete set of Instructions for removing unlocked liquidity from the Vault in the Fund
    pub fn all_instructions_fund_remove_unlocked_liquidity_vault(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        vault_name: &str,
        ui_amount: f64,
    ) -> Result<Vec<Instruction>, FarmClientError> {
        // check user accounts
        let mut inst = Vec::<Instruction>::new();
        self.check_fund_vault_custodies(
            admin_address,
            fund_name,
            vault_name,
            0.0,
            0.0,
            0.0,
            false,
            false,
            &mut inst,
        )?;

        // check if the user has unlocked balance
        if ui_amount > 0.0 {
            let fund = self.get_fund(fund_name)?;
            let lp_debt = self
                .get_vault_user_info(&fund.fund_authority, vault_name)?
                .lp_tokens_debt;
            let pool_token_decimals = self.get_vault_lp_token_decimals(vault_name)?;
            if self.tokens_to_ui_amount_with_decimals(lp_debt, pool_token_decimals) < ui_amount {
                return Err(FarmClientError::InsufficientBalance(
                    "Not enough unlocked tokens to remove".to_string(),
                ));
            }
        }

        inst.push(self.new_instruction_fund_remove_liquidity_vault(
            admin_address,
            fund_name,
            vault_name,
            ui_amount,
        )?);

        Ok(inst)
    }

    ///// private helpers
    fn build_fund_instruction(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        pool_instruction: &Instruction,
        pool_or_farm_name: &str,
        vault_type: FundVaultType,
    ) -> Result<Instruction, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        let protocol = &FarmClient::get_protocol(pool_or_farm_name)?;
        let unpacked_instruction = AmmInstruction::unpack(pool_instruction.data.as_slice())?;
        let data = match protocol {
            Protocol::Raydium => FundInstruction::AmmInstructionRaydium {
                instruction: unpacked_instruction,
            }
            .to_vec()?,
            Protocol::Orca => FundInstruction::AmmInstructionOrca {
                instruction: unpacked_instruction,
            }
            .to_vec()?,
            _ => {
                return Err(FarmClientError::ValueError(format!(
                    "Unsupported protocol {} for Fund {}",
                    protocol, fund_name
                )));
            }
        };

        let mut accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            if matches!(unpacked_instruction, AmmInstruction::UserInit { .. }) {
                AccountMeta::new(fund.fund_authority, false)
            } else {
                AccountMeta::new_readonly(fund.fund_authority, false)
            },
            AccountMeta::new_readonly(pool_instruction.program_id, false),
            if matches!(unpacked_instruction, AmmInstruction::AddLiquidity { .. })
                || matches!(unpacked_instruction, AmmInstruction::RemoveLiquidity { .. })
            {
                AccountMeta::new(
                    self.get_fund_vault_account(fund_name, pool_or_farm_name, vault_type)?,
                    false,
                )
            } else {
                AccountMeta::new_readonly(
                    self.get_fund_vault_account(fund_name, pool_or_farm_name, vault_type)?,
                    false,
                )
            },
        ];
        accounts.extend_from_slice(&pool_instruction.accounts[1..]);
        if matches!(unpacked_instruction, AmmInstruction::Swap { .. }) {
            accounts.push(AccountMeta::new_readonly(sysvar::instructions::id(), false));
        }

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }

    fn build_fund_vault_instruction(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
        vault_instruction: &Instruction,
        vault_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        let fund = self.get_fund(fund_name)?;
        let fund_ref = self.get_fund_ref(fund_name)?;

        let protocol = &FarmClient::get_protocol(vault_name)?;
        let unpacked_instruction = VaultInstruction::unpack(vault_instruction.data.as_slice())?;
        let data = match protocol {
            Protocol::Raydium => FundInstruction::VaultInstructionRaydium {
                instruction: unpacked_instruction,
            }
            .to_vec()?,
            Protocol::Orca => FundInstruction::VaultInstructionOrca {
                instruction: unpacked_instruction,
            }
            .to_vec()?,
            _ => {
                return Err(FarmClientError::ValueError(format!(
                    "Unsupported protocol {} for Fund {}",
                    protocol, fund_name
                )));
            }
        };

        let mut accounts = vec![
            AccountMeta::new_readonly(*admin_address, true),
            AccountMeta::new_readonly(fund_ref, false),
            AccountMeta::new(fund.info_account, false),
            if matches!(unpacked_instruction, VaultInstruction::UserInit { .. }) {
                AccountMeta::new(fund.fund_authority, false)
            } else {
                AccountMeta::new_readonly(fund.fund_authority, false)
            },
            AccountMeta::new_readonly(vault_instruction.program_id, false),
            if matches!(unpacked_instruction, VaultInstruction::AddLiquidity { .. })
                || matches!(
                    unpacked_instruction,
                    VaultInstruction::RemoveLiquidity { .. }
                )
            {
                AccountMeta::new(
                    self.get_fund_vault_account(fund_name, vault_name, FundVaultType::Vault)?,
                    false,
                )
            } else {
                AccountMeta::new_readonly(
                    self.get_fund_vault_account(fund_name, vault_name, FundVaultType::Vault)?,
                    false,
                )
            },
        ];
        accounts.extend_from_slice(&vault_instruction.accounts[1..]);

        Ok(Instruction {
            program_id: fund.fund_program_id,
            data,
            accounts,
        })
    }
}
