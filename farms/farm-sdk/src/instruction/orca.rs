//! Orca protocol farming instructions
//! See https://github.com/orca-so/aquafarm-sdk/blob/9ed9db0f04cf7406f1f6e9a3e316639f3d24e68c/src/instructions.ts
//! for more details and accounts references

use {
    crate::pack::check_data_len,
    arrayref::{array_mut_ref, mut_array_refs},
    solana_program::program_error::ProgramError,
};

#[repr(u8)]
#[derive(Clone, Copy, Debug)]
pub enum OrcaInstructionType {
    InitGlobalFarm,
    InitUserFarm,
    ConvertTokens,
    RevertTokens,
    Harvest,
    RemoveRewards,
    SetEmissionsPerSecond,
}

#[derive(Clone, Copy, Debug)]
pub struct OrcaUserInit {}

#[derive(Clone, Copy, Debug)]
pub struct OrcaStake {
    pub amount: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct OrcaUnstake {
    pub amount: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct OrcaHarvest {}

impl OrcaUserInit {
    pub const LEN: usize = 1;

    pub fn get_size(&self) -> usize {
        Self::LEN
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, OrcaUserInit::LEN)?;

        let output = array_mut_ref![output, 0, OrcaUserInit::LEN];
        output[0] = OrcaInstructionType::InitUserFarm as u8;

        Ok(Self::LEN)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; OrcaUserInit::LEN] = [0; OrcaUserInit::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

impl OrcaStake {
    pub const LEN: usize = 9;

    pub fn get_size(&self) -> usize {
        Self::LEN
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, OrcaStake::LEN)?;

        let output = array_mut_ref![output, 0, OrcaStake::LEN];

        let (instruction_out, amount_out) = mut_array_refs![output, 1, 8];

        instruction_out[0] = OrcaInstructionType::ConvertTokens as u8;
        *amount_out = self.amount.to_le_bytes();

        Ok(Self::LEN)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; OrcaStake::LEN] = [0; OrcaStake::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

impl OrcaUnstake {
    pub const LEN: usize = 9;

    pub fn get_size(&self) -> usize {
        Self::LEN
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, OrcaUnstake::LEN)?;

        let output = array_mut_ref![output, 0, OrcaUnstake::LEN];

        let (instruction_out, amount_out) = mut_array_refs![output, 1, 8];

        instruction_out[0] = OrcaInstructionType::RevertTokens as u8;
        *amount_out = self.amount.to_le_bytes();

        Ok(Self::LEN)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; OrcaUnstake::LEN] = [0; OrcaUnstake::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

impl OrcaHarvest {
    pub const LEN: usize = 1;

    pub fn get_size(&self) -> usize {
        Self::LEN
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, OrcaHarvest::LEN)?;

        let output = array_mut_ref![output, 0, OrcaHarvest::LEN];
        output[0] = OrcaInstructionType::Harvest as u8;

        Ok(Self::LEN)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; OrcaHarvest::LEN] = [0; OrcaHarvest::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }
}
