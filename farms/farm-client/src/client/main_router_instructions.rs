//! Solana Farm Client RefDB Instructions

use {
    crate::error::FarmClientError,
    solana_farm_sdk::{
        farm::Farm,
        fund::Fund,
        id::{main_router, main_router_multisig},
        instruction::{main_router::MainInstruction, refdb::RefDbInstruction},
        pool::Pool,
        program::multisig::Multisig,
        refdb,
        string::str_to_as64,
        token::Token,
        vault::Vault,
        ProgramIDType,
    },
    solana_sdk::{
        bpf_loader_upgradeable,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        system_program, sysvar,
    },
    std::vec::Vec,
};

use super::FarmClient;

impl FarmClient {
    /// Creates a new instruction for writing the record into on-chain RefDB
    fn new_instruction_refdb_write(
        &self,
        admin_address: &Pubkey,
        refdb_name: &str,
        record: refdb::Record,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(main_router_multisig::id(), false),
                AccountMeta::new(refdb::find_refdb_pda(refdb_name).0, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::RefDbInstruction {
            instruction: RefDbInstruction::Write { record },
        }
        .to_vec()?;

        Ok(inst)
    }

    /// Creates a new instruction for deleteing the record from on-chain RefDB
    fn new_instruction_refdb_delete(
        &self,
        admin_address: &Pubkey,
        refdb_name: &str,
        record: refdb::Record,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(main_router_multisig::id(), false),
                AccountMeta::new(refdb::find_refdb_pda(refdb_name).0, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::RefDbInstruction {
            instruction: RefDbInstruction::Delete { record },
        }
        .to_vec()?;

        Ok(inst)
    }

    /// Creates a new instruction for initializing on-chain RefDB storage
    pub fn new_instruction_refdb_init(
        &self,
        admin_address: &Pubkey,
        refdb_name: &str,
        reference_type: refdb::ReferenceType,
        max_records: u32,
        init_account: bool,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(main_router_multisig::id(), false),
                AccountMeta::new(refdb::find_refdb_pda(refdb_name).0, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::RefDbInstruction {
            instruction: RefDbInstruction::Init {
                name: str_to_as64(refdb_name)?,
                reference_type,
                max_records,
                init_account: init_account && refdb::REFDB_ONCHAIN_INIT,
            },
        }
        .to_vec()?;

        Ok(inst)
    }

    /// Creates a new instruction for removing on-chain RefDB storage
    pub fn new_instruction_refdb_drop(
        &self,
        admin_address: &Pubkey,
        refdb_name: &str,
        close_account: bool,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(main_router_multisig::id(), false),
                AccountMeta::new(refdb::find_refdb_pda(refdb_name).0, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::RefDbInstruction {
            instruction: RefDbInstruction::Drop { close_account },
        }
        .to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for removing the object reference from chain
    pub fn new_instruction_remove_reference(
        &self,
        admin_address: &Pubkey,
        storage_type: refdb::StorageType,
        object_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        let refdb_index = self
            .get_refdb_index(&storage_type.to_string(), object_name)
            .unwrap();
        self.new_instruction_refdb_delete(
            admin_address,
            &storage_type.to_string(),
            refdb::Record {
                index: refdb_index.map(|idx| idx as u32),
                counter: 0,
                tag: 0,
                name: str_to_as64(object_name)?,
                reference: refdb::Reference::Empty,
            },
        )
    }

    /// Creates a new instruction for initializing Main Router multisig with a new set of signers
    pub fn new_instruction_set_admins(
        &self,
        admin_address: &Pubkey,
        admin_signers: &[Pubkey],
        min_signatures: u8,
    ) -> Result<Instruction, FarmClientError> {
        if admin_signers.is_empty() || min_signatures == 0 {
            return Err(FarmClientError::ValueError(
                "At least one signer is required".to_string(),
            ));
        } else if min_signatures as usize > admin_signers.len()
            || admin_signers.len() > Multisig::MAX_SIGNERS
        {
            return Err(FarmClientError::ValueError(
                "Invalid number of signatures".to_string(),
            ));
        }

        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(main_router_multisig::id(), false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        for key in admin_signers {
            inst.accounts.push(AccountMeta::new_readonly(*key, false));
        }

        inst.data = MainInstruction::SetAdminSigners { min_signatures }.to_vec()?;

        Ok(inst)
    }

    /// Creates a new instruction for setting new program upgrade signers
    pub fn new_instruction_set_program_admins(
        &self,
        admin_address: &Pubkey,
        prog_id: &Pubkey,
        admin_signers: &[Pubkey],
        min_signatures: u8,
    ) -> Result<Instruction, FarmClientError> {
        if admin_signers.is_empty() || min_signatures == 0 {
            return Err(FarmClientError::ValueError(
                "At least one signer is required".to_string(),
            ));
        } else if min_signatures as usize > admin_signers.len()
            || admin_signers.len() > Multisig::MAX_SIGNERS
        {
            return Err(FarmClientError::ValueError(
                "Invalid number of signatures".to_string(),
            ));
        }

        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(self.get_program_multisig_account(prog_id)?, false),
                AccountMeta::new_readonly(*prog_id, false),
                AccountMeta::new(self.get_program_buffer_account(prog_id)?, false),
                AccountMeta::new_readonly(system_program::id(), false),
                AccountMeta::new_readonly(bpf_loader_upgradeable::id(), false),
            ],
        };

        for key in admin_signers {
            inst.accounts.push(AccountMeta::new_readonly(*key, false));
        }

        inst.data = MainInstruction::SetProgramAdminSigners { min_signatures }.to_vec()?;

        Ok(inst)
    }

    /// Creates a new instruction for setting single upgrade authority for the program
    pub fn new_instruction_set_program_single_authority(
        &self,
        admin_address: &Pubkey,
        prog_id: &Pubkey,
        upgrade_authority: &Pubkey,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        Ok(Instruction {
            program_id: main_router::id(),
            data: MainInstruction::SetProgramSingleAuthority.to_vec()?,
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(self.get_program_multisig_account(prog_id)?, false),
                AccountMeta::new_readonly(*prog_id, false),
                AccountMeta::new(self.get_program_buffer_account(prog_id)?, false),
                AccountMeta::new_readonly(*upgrade_authority, false),
                AccountMeta::new_readonly(bpf_loader_upgradeable::id(), false),
            ],
        })
    }

    /// Creates a new instruction for upgrading the program from the buffer
    pub fn new_instruction_upgrade_program(
        &self,
        admin_address: &Pubkey,
        prog_id: &Pubkey,
        source_buffer_address: &Pubkey,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        Ok(Instruction {
            program_id: main_router::id(),
            data: MainInstruction::UpgradeProgram.to_vec()?,
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(self.get_program_multisig_account(prog_id)?, false),
                AccountMeta::new(*prog_id, false),
                AccountMeta::new(self.get_program_buffer_account(prog_id)?, false),
                AccountMeta::new(*source_buffer_address, false),
                AccountMeta::new_readonly(sysvar::rent::id(), false),
                AccountMeta::new_readonly(sysvar::clock::id(), false),
                AccountMeta::new_readonly(bpf_loader_upgradeable::id(), false),
            ],
        })
    }

    /// Creates a new Instruction for recording the Program ID metadata on-chain
    pub fn new_instruction_add_program_id(
        &self,
        admin_address: &Pubkey,
        name: &str,
        program_id: &Pubkey,
        program_id_type: ProgramIDType,
        refdb_index: Option<usize>,
    ) -> Result<Instruction, FarmClientError> {
        self.new_instruction_refdb_write(
            admin_address,
            &refdb::StorageType::Program.to_string(),
            refdb::Record {
                index: refdb_index.map(|idx| idx as u32),
                counter: 0,
                tag: program_id_type as u16,
                name: str_to_as64(name)?,
                reference: refdb::Reference::Pubkey { data: *program_id },
            },
        )
    }

    /// Creates a new Instruction for removing the Program ID metadata from chain
    pub fn new_instruction_remove_program_id(
        &self,
        admin_address: &Pubkey,
        name: &str,
    ) -> Result<Instruction, FarmClientError> {
        let refdb_index = if self.get_program_id(name).is_ok() {
            self.get_refdb_index(&refdb::StorageType::Program.to_string(), name)
                .unwrap()
        } else {
            None
        };
        self.new_instruction_refdb_delete(
            admin_address,
            &refdb::StorageType::Program.to_string(),
            refdb::Record {
                index: refdb_index.map(|idx| idx as u32),
                counter: 0,
                tag: 0,
                name: str_to_as64(name)?,
                reference: refdb::Reference::Empty,
            },
        )
    }

    /// Creates a new Instruction for recording Fund's metadata on-chain
    pub fn new_instruction_add_fund(
        &self,
        admin_address: &Pubkey,
        fund: Fund,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(main_router_multisig::id(), false),
                AccountMeta::new(
                    refdb::find_refdb_pda(&refdb::StorageType::Fund.to_string()).0,
                    false,
                ),
                AccountMeta::new(
                    refdb::find_target_pda(refdb::StorageType::Fund, &fund.name).0,
                    false,
                ),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };
        inst.data = MainInstruction::AddFund { fund }.to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for removing Fund's on-chain metadata
    pub fn new_instruction_remove_fund(
        &self,
        admin_address: &Pubkey,
        fund_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        let name = str_to_as64(fund_name)?;
        let refdb_index = if let Ok(fund) = self.get_fund(fund_name) {
            fund.refdb_index
        } else {
            None
        };
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(main_router_multisig::id(), false),
                AccountMeta::new(
                    refdb::find_refdb_pda(&refdb::StorageType::Fund.to_string()).0,
                    false,
                ),
                AccountMeta::new(
                    refdb::find_target_pda(refdb::StorageType::Fund, &name).0,
                    false,
                ),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::RemoveFund { name, refdb_index }.to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for recording Vault's metadata on-chain
    pub fn new_instruction_add_vault(
        &self,
        admin_address: &Pubkey,
        vault: Vault,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(main_router_multisig::id(), false),
                AccountMeta::new(
                    refdb::find_refdb_pda(&refdb::StorageType::Vault.to_string()).0,
                    false,
                ),
                AccountMeta::new(
                    refdb::find_target_pda(refdb::StorageType::Vault, &vault.name).0,
                    false,
                ),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };
        inst.data = MainInstruction::AddVault { vault }.to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for removing Vault's on-chain metadata
    pub fn new_instruction_remove_vault(
        &self,
        admin_address: &Pubkey,
        vault_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        let name = str_to_as64(vault_name)?;
        let refdb_index = if let Ok(vault) = self.get_vault(vault_name) {
            vault.refdb_index
        } else {
            None
        };
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(main_router_multisig::id(), false),
                AccountMeta::new(
                    refdb::find_refdb_pda(&refdb::StorageType::Vault.to_string()).0,
                    false,
                ),
                AccountMeta::new(
                    refdb::find_target_pda(refdb::StorageType::Vault, &name).0,
                    false,
                ),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::RemoveVault { name, refdb_index }.to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for recording Pool's metadata on-chain
    pub fn new_instruction_add_pool(
        &self,
        admin_address: &Pubkey,
        pool: Pool,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(main_router_multisig::id(), false),
                AccountMeta::new(
                    refdb::find_refdb_pda(&refdb::StorageType::Pool.to_string()).0,
                    false,
                ),
                AccountMeta::new(
                    refdb::find_target_pda(refdb::StorageType::Pool, &pool.name).0,
                    false,
                ),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::AddPool { pool }.to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for removing Pool's on-chain metadata
    pub fn new_instruction_remove_pool(
        &self,
        admin_address: &Pubkey,
        pool_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        let name = str_to_as64(pool_name)?;
        let refdb_index = if let Ok(pool) = self.get_pool(pool_name) {
            pool.refdb_index
        } else {
            None
        };
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(main_router_multisig::id(), false),
                AccountMeta::new(
                    refdb::find_refdb_pda(&refdb::StorageType::Pool.to_string()).0,
                    false,
                ),
                AccountMeta::new(
                    refdb::find_target_pda(refdb::StorageType::Pool, &name).0,
                    false,
                ),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::RemovePool { name, refdb_index }.to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for recording Farm's metadata on-chain
    pub fn new_instruction_add_farm(
        &self,
        admin_address: &Pubkey,
        farm: Farm,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(main_router_multisig::id(), false),
                AccountMeta::new(
                    refdb::find_refdb_pda(&refdb::StorageType::Farm.to_string()).0,
                    false,
                ),
                AccountMeta::new(
                    refdb::find_target_pda(refdb::StorageType::Farm, &farm.name).0,
                    false,
                ),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::AddFarm { farm }.to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for removing Farm's on-chain metadata
    pub fn new_instruction_remove_farm(
        &self,
        admin_address: &Pubkey,
        farm_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        let name = str_to_as64(farm_name)?;
        let refdb_index = if let Ok(farm) = self.get_farm(farm_name) {
            farm.refdb_index
        } else {
            None
        };
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(main_router_multisig::id(), false),
                AccountMeta::new(
                    refdb::find_refdb_pda(&refdb::StorageType::Farm.to_string()).0,
                    false,
                ),
                AccountMeta::new(
                    refdb::find_target_pda(refdb::StorageType::Farm, &name).0,
                    false,
                ),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::RemoveFarm { name, refdb_index }.to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for recording Token's metadata on-chain
    pub fn new_instruction_add_token(
        &self,
        admin_address: &Pubkey,
        token: Token,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(main_router_multisig::id(), false),
                AccountMeta::new(
                    refdb::find_refdb_pda(&refdb::StorageType::Token.to_string()).0,
                    false,
                ),
                AccountMeta::new(
                    refdb::find_target_pda(refdb::StorageType::Token, &token.name).0,
                    false,
                ),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::AddToken { token }.to_vec()?;

        Ok(inst)
    }

    /// Creates a new Instruction for removing Token's on-chain metadata
    pub fn new_instruction_remove_token(
        &self,
        admin_address: &Pubkey,
        token_name: &str,
    ) -> Result<Instruction, FarmClientError> {
        // fill in accounts and instruction data
        let name = str_to_as64(token_name)?;
        let refdb_index = if let Ok(token) = self.get_token(token_name) {
            token.refdb_index
        } else {
            None
        };
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(main_router_multisig::id(), false),
                AccountMeta::new(
                    refdb::find_refdb_pda(&refdb::StorageType::Token.to_string()).0,
                    false,
                ),
                AccountMeta::new(
                    refdb::find_target_pda(refdb::StorageType::Token, &name).0,
                    false,
                ),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::RemoveToken { name, refdb_index }.to_vec()?;

        Ok(inst)
    }
}
