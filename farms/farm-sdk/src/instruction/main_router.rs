//! Main Router instructions.

use {
    crate::{
        farm::Farm,
        instruction::refdb::RefDbInstruction,
        pack::{check_data_len, pack_array_string64, unpack_array_string64},
        pool::Pool,
        string::ArrayString64,
        token::Token,
        vault::Vault,
    },
    arrayref::{array_mut_ref, array_ref, mut_array_refs},
    num_enum::TryFromPrimitive,
    solana_program::program_error::ProgramError,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MainInstruction {
    /// Record Vault's metadata on-chain
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be main router admin
    ///   1. [WRITE] Vault's RefDB index PDA
    ///   2. [WRITE] Vault's RefDB data PDA
    ///   3. [] Sytem program
    AddVault { vault: Vault },

    /// Delete Vault's metadata
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be main router admin
    ///   1. [WRITE] Vault's RefDB index PDA
    ///   2. [WRITE] Vault's RefDB data PDA
    ///   3. [] Sytem program
    RemoveVault { name: ArrayString64 },

    /// Record Pool's metadata on-chain
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be main router admin
    ///   1. [WRITE] Pool's RefDB index PDA
    ///   2. [WRITE] Pool's RefDB data PDA
    ///   3. [] Sytem program
    AddPool { pool: Pool },

    /// Delete Pool's metadata
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be main router admin
    ///   1. [WRITE] Pool's RefDB index PDA
    ///   2. [WRITE] Pool's RefDB data PDA
    ///   3. [] Sytem program
    RemovePool { name: ArrayString64 },

    /// Record Farm's metadata on-chain
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be main router admin
    ///   1. [WRITE] Farm's RefDB index PDA
    ///   2. [WRITE] Farm's RefDB data PDA
    ///   3. [] Sytem program
    AddFarm { farm: Farm },

    /// Delete Farm's metadata
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be main router admin
    ///   1. [WRITE] Farm's RefDB index PDA
    ///   2. [WRITE] Farm's RefDB data PDA
    ///   3. [] Sytem program
    RemoveFarm { name: ArrayString64 },

    /// Record Token's metadata on-chain
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be main router admin
    ///   1. [WRITE] Token's RefDB index PDA
    ///   2. [WRITE] Token's RefDB data PDA
    ///   3. [] Sytem program
    AddToken { token: Token },

    /// Delete Token's metadata
    ///
    /// # Account references
    ///   0. [SIGNER] Funding account, must be main router admin
    ///   1. [WRITE] Token's RefDB index PDA
    ///   2. [WRITE] Token's RefDB data PDA
    ///   3. [] Sytem program
    RemoveToken { name: ArrayString64 },

    /// Perform generic RefDB instruction
    ///
    /// # Account references are instruction specific,
    ///   see RefDbInstruction definition for more info
    RefDbInstruction { instruction: RefDbInstruction },
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum MainInstructionType {
    AddVault,
    RemoveVault,
    AddPool,
    RemovePool,
    AddFarm,
    RemoveFarm,
    AddToken,
    RemoveToken,
    RefDbInstruction,
}

impl MainInstruction {
    pub const MAX_LEN: usize = MainInstruction::max(Vault::MAX_LEN + 1, Pool::MAX_LEN + 1);
    pub const REMOVE_VAULT_LEN: usize = 65;
    pub const REMOVE_POOL_LEN: usize = 65;
    pub const REMOVE_FARM_LEN: usize = 65;
    pub const REMOVE_TOKEN_LEN: usize = 65;

    const fn max(a: usize, b: usize) -> usize {
        [a, b][(a < b) as usize]
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, 1)?;
        match self {
            Self::AddVault { vault } => self.pack_add_vault(output, vault),
            Self::RemoveVault { name } => self.pack_remove_vault(output, name),
            Self::AddPool { pool } => self.pack_add_pool(output, pool),
            Self::RemovePool { name } => self.pack_remove_pool(output, name),
            Self::AddFarm { farm } => self.pack_add_farm(output, farm),
            Self::RemoveFarm { name } => self.pack_remove_farm(output, name),
            Self::AddToken { token } => self.pack_add_token(output, token),
            Self::RemoveToken { name } => self.pack_remove_token(output, name),
            Self::RefDbInstruction { instruction } => {
                self.pack_refdb_instruction(output, instruction)
            }
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
        }
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
    ) -> Result<usize, ProgramError> {
        check_data_len(output, MainInstruction::REMOVE_VAULT_LEN)?;

        let output = array_mut_ref![output, 0, MainInstruction::REMOVE_VAULT_LEN];
        let (instruction_type_out, name_out) = mut_array_refs![output, 1, 64];

        instruction_type_out[0] = MainInstructionType::RemoveVault as u8;
        pack_array_string64(name, name_out);

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
    ) -> Result<usize, ProgramError> {
        check_data_len(output, MainInstruction::REMOVE_POOL_LEN)?;

        let output = array_mut_ref![output, 0, MainInstruction::REMOVE_POOL_LEN];
        let (instruction_type_out, name_out) = mut_array_refs![output, 1, 64];

        instruction_type_out[0] = MainInstructionType::RemovePool as u8;
        pack_array_string64(name, name_out);

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
    ) -> Result<usize, ProgramError> {
        check_data_len(output, MainInstruction::REMOVE_FARM_LEN)?;

        let output = array_mut_ref![output, 0, MainInstruction::REMOVE_FARM_LEN];
        let (instruction_type_out, name_out) = mut_array_refs![output, 1, 64];

        instruction_type_out[0] = MainInstructionType::RemoveFarm as u8;
        pack_array_string64(name, name_out);

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
    ) -> Result<usize, ProgramError> {
        check_data_len(output, MainInstruction::REMOVE_TOKEN_LEN)?;

        let output = array_mut_ref![output, 0, MainInstruction::REMOVE_TOKEN_LEN];
        let (instruction_type_out, name_out) = mut_array_refs![output, 1, 64];

        instruction_type_out[0] = MainInstructionType::RemoveToken as u8;
        pack_array_string64(name, name_out);

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

    fn unpack_add_vault(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        let vault = Vault::unpack(&input[1..])?;
        Ok(Self::AddVault { vault })
    }

    fn unpack_remove_vault(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        check_data_len(input, MainInstruction::REMOVE_VAULT_LEN)?;
        let input = array_ref![input, 1, MainInstruction::REMOVE_VAULT_LEN - 1];
        Ok(Self::RemoveVault {
            name: unpack_array_string64(input)?,
        })
    }

    fn unpack_add_pool(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        let pool = Pool::unpack(&input[1..])?;
        Ok(Self::AddPool { pool })
    }

    fn unpack_remove_pool(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        check_data_len(input, MainInstruction::REMOVE_POOL_LEN)?;
        let input = array_ref![input, 1, MainInstruction::REMOVE_POOL_LEN - 1];
        Ok(Self::RemovePool {
            name: unpack_array_string64(input)?,
        })
    }

    fn unpack_add_farm(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        let farm = Farm::unpack(&input[1..])?;
        Ok(Self::AddFarm { farm })
    }

    fn unpack_remove_farm(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        check_data_len(input, MainInstruction::REMOVE_FARM_LEN)?;
        let input = array_ref![input, 1, MainInstruction::REMOVE_FARM_LEN - 1];
        Ok(Self::RemoveFarm {
            name: unpack_array_string64(input)?,
        })
    }

    fn unpack_add_token(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        let token = Token::unpack(&input[1..])?;
        Ok(Self::AddToken { token })
    }

    fn unpack_remove_token(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        check_data_len(input, MainInstruction::REMOVE_TOKEN_LEN)?;
        let input = array_ref![input, 1, MainInstruction::REMOVE_TOKEN_LEN - 1];
        Ok(Self::RemoveToken {
            name: unpack_array_string64(input)?,
        })
    }

    fn unpack_refdb_instruction(input: &[u8]) -> Result<MainInstruction, ProgramError> {
        let instruction = RefDbInstruction::unpack(&input[1..])?;
        Ok(Self::RefDbInstruction { instruction })
    }
}

impl std::fmt::Display for MainInstructionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            MainInstructionType::AddVault => write!(f, "AddVault"),
            MainInstructionType::RemoveVault => write!(f, "RemoveVault"),
            MainInstructionType::AddPool => write!(f, "AddPool"),
            MainInstructionType::RemovePool => write!(f, "RemovePool"),
            MainInstructionType::AddFarm => write!(f, "AddFarm"),
            MainInstructionType::RemoveFarm => write!(f, "RemoveFarm"),
            MainInstructionType::AddToken => write!(f, "AddToken"),
            MainInstructionType::RemoveToken => write!(f, "RemoveToken"),
            MainInstructionType::RefDbInstruction => write!(f, "RefDbInstruction"),
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
