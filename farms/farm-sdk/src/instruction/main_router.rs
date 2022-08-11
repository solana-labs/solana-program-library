//! Main Router instructions.

use {
    crate::{
        farm::Farm,
        fund::Fund,
        instruction::refdb::RefDbInstruction,
        pack::{
            check_data_len, pack_array_string64, pack_option_u32, unpack_array_string64,
            unpack_option_u32,
        },
        pool::Pool,
        string::ArrayString64,
        token::Token,
        traits::Packed,
        vault::Vault,
    },
    arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs},
    num_enum::TryFromPrimitive,
    solana_program::program_error::ProgramError,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MainInstruction {
    /// Record Fund's metadata on-chain
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be one of the multisig signers or main router admin if no multisig
    ///   1. [WRITE] Multisig PDA address, must be main_router_multisig::id()
    ///   2. [WRITE] Fund's RefDB refdb_index PDA
    ///   3. [WRITE] Fund's RefDB data PDA
    ///   4. [] Sytem program
    AddFund { fund: Fund },

    /// Delete Fund's metadata
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be one of the multisig signers or main router admin if no multisig
    ///   1. [WRITE] Multisig PDA address, must be main_router_multisig::id()
    ///   2. [WRITE] Fund's RefDB refdb_index PDA
    ///   3. [WRITE] Fund's RefDB data PDA
    ///   4. [] Sytem program
    RemoveFund {
        name: ArrayString64,
        refdb_index: Option<u32>,
    },

    /// Record Vault's metadata on-chain
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be one of the multisig signers or main router admin if no multisig
    ///   1. [WRITE] Multisig PDA address, must be main_router_multisig::id()
    ///   2. [WRITE] Vault's RefDB refdb_index PDA
    ///   3. [WRITE] Vault's RefDB data PDA
    ///   4. [] Sytem program
    AddVault { vault: Vault },

    /// Delete Vault's metadata
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be one of the multisig signers or main router admin if no multisig
    ///   1. [WRITE] Multisig PDA address, must be main_router_multisig::id()
    ///   2. [WRITE] Vault's RefDB refdb_index PDA
    ///   3. [WRITE] Vault's RefDB data PDA
    ///   4. [] Sytem program
    RemoveVault {
        name: ArrayString64,
        refdb_index: Option<u32>,
    },

    /// Record Pool's metadata on-chain
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be one of the multisig signers or main router admin if no multisig
    ///   1. [WRITE] Multisig PDA address, must be main_router_multisig::id()
    ///   2. [WRITE] Pool's RefDB refdb_index PDA
    ///   3. [WRITE] Pool's RefDB data PDA
    ///   4. [] Sytem program
    AddPool { pool: Pool },

    /// Delete Pool's metadata
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be one of the multisig signers or main router admin if no multisig
    ///   1. [WRITE] Multisig PDA address, must be main_router_multisig::id()
    ///   2. [WRITE] Pool's RefDB refdb_index PDA
    ///   3. [WRITE] Pool's RefDB data PDA
    ///   4. [] Sytem program
    RemovePool {
        name: ArrayString64,
        refdb_index: Option<u32>,
    },

    /// Record Farm's metadata on-chain
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be one of the multisig signers or main router admin if no multisig
    ///   1. [WRITE] Multisig PDA address, must be main_router_multisig::id()
    ///   2. [WRITE] Farm's RefDB refdb_index PDA
    ///   3. [WRITE] Farm's RefDB data PDA
    ///   4. [] Sytem program
    AddFarm { farm: Farm },

    /// Delete Farm's metadata
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be one of the multisig signers or main router admin if no multisig
    ///   1. [WRITE] Multisig PDA address, must be main_router_multisig::id()
    ///   2. [WRITE] Farm's RefDB refdb_index PDA
    ///   3. [WRITE] Farm's RefDB data PDA
    ///   4. [] Sytem program
    RemoveFarm {
        name: ArrayString64,
        refdb_index: Option<u32>,
    },

    /// Record Token's metadata on-chain
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be one of the multisig signers or main router admin if no multisig
    ///   1. [WRITE] Multisig PDA address, must be main_router_multisig::id()
    ///   2. [WRITE] Token's RefDB refdb_index PDA
    ///   3. [WRITE] Token's RefDB data PDA
    ///   4. [] Sytem program
    AddToken { token: Token },

    /// Delete Token's metadata
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be one of the multisig signers or main router admin if no multisig
    ///   1. [WRITE] Multisig PDA address, must be main_router_multisig::id()
    ///   2. [WRITE] Token's RefDB refdb_index PDA
    ///   3. [WRITE] Token's RefDB data PDA
    ///   4. [] Sytem program
    RemoveToken {
        name: ArrayString64,
        refdb_index: Option<u32>,
    },

    /// Perform generic RefDB instruction
    ///
    /// # Account references are instruction specific,
    ///   see RefDbInstruction definition for more info
    RefDbInstruction { instruction: RefDbInstruction },

    /// Initialize Main Router multisig with a new set of admin signatures
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be one of the multisig signers or main router admin if no multisig
    ///   1. [WRITE] Multisig PDA address, must be main_router_multisig::id()
    ///   2. [] Sytem program
    ///   3. [] First signer
    ///  ... [] Extra signers, up to Multisig::MAX_SIGNERS
    SetAdminSigners { min_signatures: u8 },

    /// Initialize program upgrade authority multisig with a new set of admin signatures
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be one of the multisig signers or upgrade authority if no multisig
    ///   1. [WRITE] Multisig PDA address, must be get_program_multisig_account()
    ///   2. [] Program address
    ///   3. [WRITE] Program data buffer address
    ///   4. [] Sytem program
    ///   5. [] BPF Loader program
    ///   6. [] First signer
    ///  ... [] Extra signers, up to Multisig::MAX_SIGNERS
    SetProgramAdminSigners { min_signatures: u8 },

    /// Set single upgrade authority for the program removing multisig if present
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be one of the multisig signers or upgrade authority if no multisig
    ///   1. [WRITE] Multisig PDA address, must be get_program_multisig_account()
    ///   2. [] Program address
    ///   3. [WRITE] Program data buffer address
    ///   4. [] New upgrade authority
    ///   5. [] BPF Loader program
    SetProgramSingleAuthority,

    /// Upgrade the program from the buffer
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be one of the multisig signers or upgrade authority if no multisig
    ///   1. [WRITE] Multisig PDA address, must be get_program_multisig_account()
    ///   2. [WRITE] Program address
    ///   3. [WRITE] Program data buffer address
    ///   4. [WRITE] Source data buffer address
    ///   5. [] Rent sysvar
    ///   6. [] Clock sysvar
    ///   7. [] BPF Loader program
    UpgradeProgram,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum MainInstructionType {
    AddFund,
    RemoveFund,
    AddVault,
    RemoveVault,
    AddPool,
    RemovePool,
    AddFarm,
    RemoveFarm,
    AddToken,
    RemoveToken,
    RefDbInstruction,
    SetAdminSigners,
    SetProgramAdminSigners,
    SetProgramSingleAuthority,
    UpgradeProgram,
}

impl MainInstruction {
    pub const MAX_LEN: usize = MainInstruction::max(Vault::MAX_LEN + 1, Pool::MAX_LEN + 1);
    pub const REMOVE_FUND_LEN: usize = 70;
    pub const REMOVE_VAULT_LEN: usize = 70;
    pub const REMOVE_POOL_LEN: usize = 70;
    pub const REMOVE_FARM_LEN: usize = 70;
    pub const REMOVE_TOKEN_LEN: usize = 70;
    pub const SET_ADMIN_SIGNERS_LEN: usize = 2;
    pub const SET_PROGRAM_ADMIN_SIGNERS_LEN: usize = 2;
    pub const SET_PROGRAM_SINGLE_AUTHORITY_LEN: usize = 1;
    pub const UPGRADE_PROGRAM_LEN: usize = 1;

    const fn max(a: usize, b: usize) -> usize {
        [a, b][(a < b) as usize]
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, 1)?;
        match self {
            Self::AddFund { fund } => self.pack_add_fund(output, fund),
            Self::RemoveFund { name, refdb_index } => {
                self.pack_remove_fund(output, name, refdb_index)
            }
            Self::AddVault { vault } => self.pack_add_vault(output, vault),
            Self::RemoveVault { name, refdb_index } => {
                self.pack_remove_vault(output, name, refdb_index)
            }
            Self::AddPool { pool } => self.pack_add_pool(output, pool),
            Self::RemovePool { name, refdb_index } => {
                self.pack_remove_pool(output, name, refdb_index)
            }
            Self::AddFarm { farm } => self.pack_add_farm(output, farm),
            Self::RemoveFarm { name, refdb_index } => {
                self.pack_remove_farm(output, name, refdb_index)
            }
            Self::AddToken { token } => self.pack_add_token(output, token),
            Self::RemoveToken { name, refdb_index } => {
                self.pack_remove_token(output, name, refdb_index)
            }
            Self::RefDbInstruction { instruction } => {
                self.pack_refdb_instruction(output, instruction)
            }
            Self::SetAdminSigners { min_signatures } => {
                self.pack_set_admin_signers(output, *min_signatures)
            }
            Self::SetProgramAdminSigners { min_signatures } => {
                self.pack_set_program_admin_signers(output, *min_signatures)
            }
            Self::SetProgramSingleAuthority => self.pack_set_program_single_authority(output),
            Self::UpgradeProgram => self.pack_upgrade_program(output),
        }
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; MainInstruction::MAX_LEN] = [0; MainInstruction::MAX_LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    pub fn unpack(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        check_data_len(input, 1)?;
        let instruction_type = MainInstructionType::try_from_primitive(input[0])
            .or(Err(ProgramError::InvalidInstructionData))?;
        match instruction_type {
            MainInstructionType::AddFund => MainInstruction::unpack_add_fund(input),
            MainInstructionType::RemoveFund => MainInstruction::unpack_remove_fund(input),
            MainInstructionType::AddVault => MainInstruction::unpack_add_vault(input),
            MainInstructionType::RemoveVault => MainInstruction::unpack_remove_vault(input),
            MainInstructionType::AddPool => MainInstruction::unpack_add_pool(input),
            MainInstructionType::RemovePool => MainInstruction::unpack_remove_pool(input),
            MainInstructionType::AddFarm => MainInstruction::unpack_add_farm(input),
            MainInstructionType::RemoveFarm => MainInstruction::unpack_remove_farm(input),
            MainInstructionType::AddToken => MainInstruction::unpack_add_token(input),
            MainInstructionType::RemoveToken => MainInstruction::unpack_remove_token(input),
            MainInstructionType::RefDbInstruction => {
                MainInstruction::unpack_refdb_instruction(input)
            }
            MainInstructionType::SetAdminSigners => {
                MainInstruction::unpack_set_admin_signers(input)
            }
            MainInstructionType::SetProgramAdminSigners => {
                MainInstruction::unpack_set_program_admin_signers(input)
            }
            MainInstructionType::SetProgramSingleAuthority => {
                MainInstruction::unpack_set_program_single_authority(input)
            }
            MainInstructionType::UpgradeProgram => MainInstruction::unpack_upgrade_program(input),
        }
    }

    fn pack_add_fund(&self, output: &mut [u8], fund: &Fund) -> Result<usize, ProgramError> {
        let packed = fund.pack(&mut output[1..])?;
        let instruction_type_out = array_mut_ref![output, 0, 1];
        instruction_type_out[0] = MainInstructionType::AddFund as u8;

        Ok(packed + 1)
    }

    fn pack_remove_fund(
        &self,
        output: &mut [u8],
        name: &ArrayString64,
        refdb_index: &Option<u32>,
    ) -> Result<usize, ProgramError> {
        check_data_len(output, MainInstruction::REMOVE_FUND_LEN)?;

        let output = array_mut_ref![output, 0, MainInstruction::REMOVE_FUND_LEN];
        let (instruction_type_out, name_out, refdb_index_out) = mut_array_refs![output, 1, 64, 5];

        instruction_type_out[0] = MainInstructionType::RemoveFund as u8;
        pack_array_string64(name, name_out);
        pack_option_u32(*refdb_index, refdb_index_out);

        Ok(MainInstruction::REMOVE_FUND_LEN)
    }

    fn pack_add_vault(&self, output: &mut [u8], vault: &Vault) -> Result<usize, ProgramError> {
        let packed = vault.pack(&mut output[1..])?;
        let instruction_type_out = array_mut_ref![output, 0, 1];
        instruction_type_out[0] = MainInstructionType::AddVault as u8;

        Ok(packed + 1)
    }

    fn pack_remove_vault(
        &self,
        output: &mut [u8],
        name: &ArrayString64,
        refdb_index: &Option<u32>,
    ) -> Result<usize, ProgramError> {
        check_data_len(output, MainInstruction::REMOVE_VAULT_LEN)?;

        let output = array_mut_ref![output, 0, MainInstruction::REMOVE_VAULT_LEN];
        let (instruction_type_out, name_out, refdb_index_out) = mut_array_refs![output, 1, 64, 5];

        instruction_type_out[0] = MainInstructionType::RemoveVault as u8;
        pack_array_string64(name, name_out);
        pack_option_u32(*refdb_index, refdb_index_out);

        Ok(MainInstruction::REMOVE_VAULT_LEN)
    }

    fn pack_add_pool(&self, output: &mut [u8], pool: &Pool) -> Result<usize, ProgramError> {
        let packed = pool.pack(&mut output[1..])?;
        let instruction_type_out = array_mut_ref![output, 0, 1];
        instruction_type_out[0] = MainInstructionType::AddPool as u8;

        Ok(packed + 1)
    }

    fn pack_remove_pool(
        &self,
        output: &mut [u8],
        name: &ArrayString64,
        refdb_index: &Option<u32>,
    ) -> Result<usize, ProgramError> {
        check_data_len(output, MainInstruction::REMOVE_POOL_LEN)?;

        let output = array_mut_ref![output, 0, MainInstruction::REMOVE_POOL_LEN];
        let (instruction_type_out, name_out, refdb_index_out) = mut_array_refs![output, 1, 64, 5];

        instruction_type_out[0] = MainInstructionType::RemovePool as u8;
        pack_array_string64(name, name_out);
        pack_option_u32(*refdb_index, refdb_index_out);

        Ok(MainInstruction::REMOVE_POOL_LEN)
    }

    fn pack_add_farm(&self, output: &mut [u8], farm: &Farm) -> Result<usize, ProgramError> {
        let packed = farm.pack(&mut output[1..])?;
        let instruction_type_out = array_mut_ref![output, 0, 1];
        instruction_type_out[0] = MainInstructionType::AddFarm as u8;

        Ok(packed + 1)
    }

    fn pack_remove_farm(
        &self,
        output: &mut [u8],
        name: &ArrayString64,
        refdb_index: &Option<u32>,
    ) -> Result<usize, ProgramError> {
        check_data_len(output, MainInstruction::REMOVE_FARM_LEN)?;

        let output = array_mut_ref![output, 0, MainInstruction::REMOVE_FARM_LEN];
        let (instruction_type_out, name_out, refdb_index_out) = mut_array_refs![output, 1, 64, 5];

        instruction_type_out[0] = MainInstructionType::RemoveFarm as u8;
        pack_array_string64(name, name_out);
        pack_option_u32(*refdb_index, refdb_index_out);

        Ok(MainInstruction::REMOVE_FARM_LEN)
    }

    fn pack_add_token(&self, output: &mut [u8], token: &Token) -> Result<usize, ProgramError> {
        let packed = token.pack(&mut output[1..])?;
        let instruction_type_out = array_mut_ref![output, 0, 1];
        instruction_type_out[0] = MainInstructionType::AddToken as u8;

        Ok(packed + 1)
    }

    fn pack_remove_token(
        &self,
        output: &mut [u8],
        name: &ArrayString64,
        refdb_index: &Option<u32>,
    ) -> Result<usize, ProgramError> {
        check_data_len(output, MainInstruction::REMOVE_TOKEN_LEN)?;

        let output = array_mut_ref![output, 0, MainInstruction::REMOVE_TOKEN_LEN];
        let (instruction_type_out, name_out, refdb_index_out) = mut_array_refs![output, 1, 64, 5];

        instruction_type_out[0] = MainInstructionType::RemoveToken as u8;
        pack_array_string64(name, name_out);
        pack_option_u32(*refdb_index, refdb_index_out);

        Ok(MainInstruction::REMOVE_TOKEN_LEN)
    }

    fn pack_refdb_instruction(
        &self,
        output: &mut [u8],
        instruction: &RefDbInstruction,
    ) -> Result<usize, ProgramError> {
        let packed = instruction.pack(&mut output[1..])?;
        let instruction_type_out = array_mut_ref![output, 0, 1];
        instruction_type_out[0] = MainInstructionType::RefDbInstruction as u8;

        Ok(packed + 1)
    }

    fn pack_set_admin_signers(
        &self,
        output: &mut [u8],
        min_signatures: u8,
    ) -> Result<usize, ProgramError> {
        check_data_len(output, MainInstruction::SET_ADMIN_SIGNERS_LEN)?;

        let output = array_mut_ref![output, 0, MainInstruction::SET_ADMIN_SIGNERS_LEN];
        let (instruction_type_out, min_signatures_out) = mut_array_refs![output, 1, 1];

        instruction_type_out[0] = MainInstructionType::SetAdminSigners as u8;
        min_signatures_out[0] = min_signatures;

        Ok(MainInstruction::SET_ADMIN_SIGNERS_LEN)
    }

    fn pack_set_program_admin_signers(
        &self,
        output: &mut [u8],
        min_signatures: u8,
    ) -> Result<usize, ProgramError> {
        check_data_len(output, MainInstruction::SET_PROGRAM_ADMIN_SIGNERS_LEN)?;

        let output = array_mut_ref![output, 0, MainInstruction::SET_PROGRAM_ADMIN_SIGNERS_LEN];
        let (instruction_type_out, min_signatures_out) = mut_array_refs![output, 1, 1];

        instruction_type_out[0] = MainInstructionType::SetProgramAdminSigners as u8;
        min_signatures_out[0] = min_signatures;

        Ok(MainInstruction::SET_PROGRAM_ADMIN_SIGNERS_LEN)
    }

    fn pack_set_program_single_authority(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, MainInstruction::SET_PROGRAM_SINGLE_AUTHORITY_LEN)?;
        output[0] = MainInstructionType::SetProgramSingleAuthority as u8;

        Ok(MainInstruction::SET_PROGRAM_SINGLE_AUTHORITY_LEN)
    }

    fn pack_upgrade_program(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, MainInstruction::UPGRADE_PROGRAM_LEN)?;
        output[0] = MainInstructionType::UpgradeProgram as u8;

        Ok(MainInstruction::UPGRADE_PROGRAM_LEN)
    }

    fn unpack_add_fund(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        let fund = Fund::unpack(&input[1..])?;
        Ok(Self::AddFund { fund })
    }

    fn unpack_remove_fund(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        check_data_len(input, MainInstruction::REMOVE_FUND_LEN)?;

        let input = array_ref![input, 1, MainInstruction::REMOVE_FUND_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (name, refdb_index) = array_refs![input, 64, 5];

        Ok(Self::RemoveFund {
            name: unpack_array_string64(name)?,
            refdb_index: unpack_option_u32(refdb_index)?,
        })
    }

    fn unpack_add_vault(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        let vault = Vault::unpack(&input[1..])?;
        Ok(Self::AddVault { vault })
    }

    fn unpack_remove_vault(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        check_data_len(input, MainInstruction::REMOVE_VAULT_LEN)?;

        let input = array_ref![input, 1, MainInstruction::REMOVE_VAULT_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (name, refdb_index) = array_refs![input, 64, 5];

        Ok(Self::RemoveVault {
            name: unpack_array_string64(name)?,
            refdb_index: unpack_option_u32(refdb_index)?,
        })
    }

    fn unpack_add_pool(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        let pool = Pool::unpack(&input[1..])?;
        Ok(Self::AddPool { pool })
    }

    fn unpack_remove_pool(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        check_data_len(input, MainInstruction::REMOVE_POOL_LEN)?;

        let input = array_ref![input, 1, MainInstruction::REMOVE_POOL_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (name, refdb_index) = array_refs![input, 64, 5];

        Ok(Self::RemovePool {
            name: unpack_array_string64(name)?,
            refdb_index: unpack_option_u32(refdb_index)?,
        })
    }

    fn unpack_add_farm(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        let farm = Farm::unpack(&input[1..])?;
        Ok(Self::AddFarm { farm })
    }

    fn unpack_remove_farm(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        check_data_len(input, MainInstruction::REMOVE_FARM_LEN)?;

        let input = array_ref![input, 1, MainInstruction::REMOVE_FARM_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (name, refdb_index) = array_refs![input, 64, 5];

        Ok(Self::RemoveFarm {
            name: unpack_array_string64(name)?,
            refdb_index: unpack_option_u32(refdb_index)?,
        })
    }

    fn unpack_add_token(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        let token = Token::unpack(&input[1..])?;
        Ok(Self::AddToken { token })
    }

    fn unpack_remove_token(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        check_data_len(input, MainInstruction::REMOVE_TOKEN_LEN)?;

        let input = array_ref![input, 1, MainInstruction::REMOVE_TOKEN_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (name, refdb_index) = array_refs![input, 64, 5];

        Ok(Self::RemoveToken {
            name: unpack_array_string64(name)?,
            refdb_index: unpack_option_u32(refdb_index)?,
        })
    }

    fn unpack_refdb_instruction(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        let instruction = RefDbInstruction::unpack(&input[1..])?;
        Ok(Self::RefDbInstruction { instruction })
    }

    fn unpack_set_admin_signers(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        check_data_len(input, MainInstruction::SET_ADMIN_SIGNERS_LEN)?;

        let input = array_ref![input, 1, MainInstruction::SET_ADMIN_SIGNERS_LEN - 1];

        Ok(Self::SetAdminSigners {
            min_signatures: input[0],
        })
    }

    fn unpack_set_program_admin_signers(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        check_data_len(input, MainInstruction::SET_PROGRAM_ADMIN_SIGNERS_LEN)?;

        let input = array_ref![input, 1, MainInstruction::SET_PROGRAM_ADMIN_SIGNERS_LEN - 1];

        Ok(Self::SetProgramAdminSigners {
            min_signatures: input[0],
        })
    }

    fn unpack_set_program_single_authority(_input: &[u8]) -> Result<MainInstruction, ProgramError> {
        Ok(Self::SetProgramSingleAuthority)
    }

    fn unpack_upgrade_program(_input: &[u8]) -> Result<MainInstruction, ProgramError> {
        Ok(Self::UpgradeProgram)
    }
}

impl std::fmt::Display for MainInstructionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            MainInstructionType::AddFund => write!(f, "AddFund"),
            MainInstructionType::RemoveFund => write!(f, "RemoveFund"),
            MainInstructionType::AddVault => write!(f, "AddVault"),
            MainInstructionType::RemoveVault => write!(f, "RemoveVault"),
            MainInstructionType::AddPool => write!(f, "AddPool"),
            MainInstructionType::RemovePool => write!(f, "RemovePool"),
            MainInstructionType::AddFarm => write!(f, "AddFarm"),
            MainInstructionType::RemoveFarm => write!(f, "RemoveFarm"),
            MainInstructionType::AddToken => write!(f, "AddToken"),
            MainInstructionType::RemoveToken => write!(f, "RemoveToken"),
            MainInstructionType::RefDbInstruction => write!(f, "RefDbInstruction"),
            MainInstructionType::SetAdminSigners => write!(f, "SetAdminSigners"),
            MainInstructionType::SetProgramAdminSigners => write!(f, "SetProgramAdminSigners"),
            MainInstructionType::SetProgramSingleAuthority => {
                write!(f, "SetProgramSingleAuthority")
            }
            MainInstructionType::UpgradeProgram => write!(f, "UpgradeProgram"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::{PoolRoute, PoolType};
    use crate::string::ArrayString64;
    use solana_program::pubkey::Pubkey;

    #[test]
    fn test_vec_serialization() {
        let ri1 = MainInstruction::AddPool {
            pool: Pool {
                name: ArrayString64::from_utf8("test").unwrap(),
                version: 2,
                pool_type: PoolType::Amm,
                official: true,
                refdb_index: Some(1),
                refdb_counter: 2,
                token_a_ref: Some(Pubkey::new_unique()),
                token_b_ref: Some(Pubkey::new_unique()),
                lp_token_ref: Some(Pubkey::new_unique()),
                token_a_account: None,
                token_b_account: None,
                router_program_id: Pubkey::new_unique(),
                pool_program_id: Pubkey::new_unique(),
                route: PoolRoute::Raydium {
                    amm_id: Pubkey::new_unique(),
                    amm_authority: Pubkey::new_unique(),
                    amm_open_orders: Pubkey::new_unique(),
                    amm_target: Pubkey::new_unique(),
                    pool_withdraw_queue: Pubkey::new_unique(),
                    pool_temp_lp_token_account: Pubkey::new_unique(),
                    serum_program_id: Pubkey::new_unique(),
                    serum_market: Pubkey::new_unique(),
                    serum_coin_vault_account: Pubkey::new_unique(),
                    serum_pc_vault_account: Pubkey::new_unique(),
                    serum_vault_signer: Pubkey::new_unique(),
                    serum_bids: Some(Pubkey::new_unique()),
                    serum_asks: Some(Pubkey::new_unique()),
                    serum_event_queue: Some(Pubkey::new_unique()),
                },
            },
        };

        let vec = ri1.to_vec().unwrap();

        let ri2 = MainInstruction::unpack(&vec[..]).unwrap();

        assert_eq!(ri1, ri2);
    }
}
