//! Solana Farm Client RefDB Instructions

use {
    crate::error::FarmClientError,
    solana_farm_sdk::{
        farm::Farm,
        id::{main_router, ProgramIDType},
        instruction::{main_router::MainInstruction, refdb::RefDbInstruction},
        pool::Pool,
        program::pda::{find_refdb_pda, find_target_pda},
        refdb,
        string::str_to_as64,
        token::Token,
        vault::Vault,
    },
    solana_sdk::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        system_program,
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
                AccountMeta::new(find_refdb_pda(refdb_name).0, false),
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
                AccountMeta::new(find_refdb_pda(refdb_name).0, false),
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
                AccountMeta::new(find_refdb_pda(refdb_name).0, false),
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
                AccountMeta::new(find_refdb_pda(refdb_name).0, false),
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
        refdb_index: Option<usize>,
    ) -> Result<Instruction, FarmClientError> {
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
        refdb_index: Option<usize>,
    ) -> Result<Instruction, FarmClientError> {
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
                AccountMeta::new(
                    find_refdb_pda(&refdb::StorageType::Vault.to_string()).0,
                    false,
                ),
                AccountMeta::new(
                    find_target_pda(refdb::StorageType::Vault, &vault.name).0,
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
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(
                    find_refdb_pda(&refdb::StorageType::Vault.to_string()).0,
                    false,
                ),
                AccountMeta::new(find_target_pda(refdb::StorageType::Vault, &name).0, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::RemoveVault { name }.to_vec()?;

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
                AccountMeta::new(
                    find_refdb_pda(&refdb::StorageType::Pool.to_string()).0,
                    false,
                ),
                AccountMeta::new(
                    find_target_pda(refdb::StorageType::Pool, &pool.name).0,
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
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(
                    find_refdb_pda(&refdb::StorageType::Pool.to_string()).0,
                    false,
                ),
                AccountMeta::new(find_target_pda(refdb::StorageType::Pool, &name).0, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::RemovePool { name }.to_vec()?;

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
                AccountMeta::new(
                    find_refdb_pda(&refdb::StorageType::Farm.to_string()).0,
                    false,
                ),
                AccountMeta::new(
                    find_target_pda(refdb::StorageType::Farm, &farm.name).0,
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
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(
                    find_refdb_pda(&refdb::StorageType::Farm.to_string()).0,
                    false,
                ),
                AccountMeta::new(find_target_pda(refdb::StorageType::Farm, &name).0, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::RemoveFarm { name }.to_vec()?;

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
                AccountMeta::new(
                    find_refdb_pda(&refdb::StorageType::Token.to_string()).0,
                    false,
                ),
                AccountMeta::new(
                    find_target_pda(refdb::StorageType::Token, &token.name).0,
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
        let mut inst = Instruction {
            program_id: main_router::id(),
            data: Vec::<u8>::new(),
            accounts: vec![
                AccountMeta::new_readonly(*admin_address, true),
                AccountMeta::new(
                    find_refdb_pda(&refdb::StorageType::Token.to_string()).0,
                    false,
                ),
                AccountMeta::new(find_target_pda(refdb::StorageType::Token, &name).0, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
        };

        inst.data = MainInstruction::RemoveToken { name }.to_vec()?;

        Ok(inst)
    }
}
