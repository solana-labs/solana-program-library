//! Raydium protocol native instructions
//! See https://github.com/raydium-io/raydium-contract-instructions/blob/master/amm_instruction.rs
//! for more details and accounts references

use {
    crate::pack::check_data_len,
    arrayref::{array_mut_ref, mut_array_refs},
    solana_program::program_error::ProgramError,
};

#[derive(Clone, Copy, Debug)]
pub struct RaydiumAddLiquidity {
    pub instruction: u8,
    pub max_coin_token_amount: u64,
    pub max_pc_token_amount: u64,
    pub base_side: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct RaydiumRemoveLiquidity {
    pub instruction: u8,
    pub amount: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct RaydiumSwap {
    pub instruction: u8,
    pub amount_in: u64,
    pub min_amount_out: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct RaydiumStake {
    pub instruction: u8,
    pub amount: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct RaydiumUnstake {
    pub instruction: u8,
    pub amount: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct RaydiumHarvest {
    pub instruction: u8,
}

impl RaydiumAddLiquidity {
    pub const LEN: usize = 25;

    pub fn get_size(&self) -> usize {
        RaydiumAddLiquidity::LEN
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, RaydiumAddLiquidity::LEN)?;

        let output = array_mut_ref![output, 0, RaydiumAddLiquidity::LEN];

        let (instruction_out, max_coin_token_amount_out, max_pc_token_amount_out, base_side_out) =
            mut_array_refs![output, 1, 8, 8, 8];

        instruction_out[0] = self.instruction as u8;
        *max_coin_token_amount_out = self.max_coin_token_amount.to_le_bytes();
        *max_pc_token_amount_out = self.max_pc_token_amount.to_le_bytes();
        *base_side_out = self.base_side.to_le_bytes();

        Ok(RaydiumAddLiquidity::LEN)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; RaydiumAddLiquidity::LEN] = [0; RaydiumAddLiquidity::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

impl RaydiumRemoveLiquidity {
    pub const LEN: usize = 9;

    pub fn get_size(&self) -> usize {
        RaydiumRemoveLiquidity::LEN
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, RaydiumRemoveLiquidity::LEN)?;

        let output = array_mut_ref![output, 0, RaydiumRemoveLiquidity::LEN];

        let (instruction_out, amount_out) = mut_array_refs![output, 1, 8];

        instruction_out[0] = self.instruction as u8;
        *amount_out = self.amount.to_le_bytes();

        Ok(RaydiumRemoveLiquidity::LEN)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; RaydiumRemoveLiquidity::LEN] = [0; RaydiumRemoveLiquidity::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

impl RaydiumSwap {
    pub const LEN: usize = 17;

    pub fn get_size(&self) -> usize {
        RaydiumSwap::LEN
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, RaydiumSwap::LEN)?;

        let output = array_mut_ref![output, 0, RaydiumSwap::LEN];

        let (instruction_out, amount_in_out, min_amount_out_out) = mut_array_refs![output, 1, 8, 8];

        instruction_out[0] = self.instruction as u8;
        *amount_in_out = self.amount_in.to_le_bytes();
        *min_amount_out_out = self.min_amount_out.to_le_bytes();

        Ok(RaydiumSwap::LEN)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; RaydiumSwap::LEN] = [0; RaydiumSwap::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

impl RaydiumStake {
    pub const LEN: usize = 9;

    pub fn get_size(&self) -> usize {
        RaydiumStake::LEN
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, RaydiumStake::LEN)?;

        let output = array_mut_ref![output, 0, RaydiumStake::LEN];

        let (instruction_out, amount_out) = mut_array_refs![output, 1, 8];

        instruction_out[0] = self.instruction as u8;
        *amount_out = self.amount.to_le_bytes();

        Ok(RaydiumStake::LEN)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; RaydiumStake::LEN] = [0; RaydiumStake::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

impl RaydiumUnstake {
    pub const LEN: usize = 9;

    pub fn get_size(&self) -> usize {
        RaydiumUnstake::LEN
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, RaydiumUnstake::LEN)?;

        let output = array_mut_ref![output, 0, RaydiumUnstake::LEN];

        let (instruction_out, amount_out) = mut_array_refs![output, 1, 8];

        instruction_out[0] = self.instruction as u8;
        *amount_out = self.amount.to_le_bytes();

        Ok(RaydiumUnstake::LEN)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; RaydiumUnstake::LEN] = [0; RaydiumUnstake::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

impl RaydiumHarvest {
    pub const LEN: usize = 1;

    pub fn get_size(&self) -> usize {
        RaydiumHarvest::LEN
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, RaydiumHarvest::LEN)?;

        let output = array_mut_ref![output, 0, RaydiumHarvest::LEN];
        output[0] = self.instruction as u8;

        Ok(RaydiumHarvest::LEN)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; RaydiumHarvest::LEN] = [0; RaydiumHarvest::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }
}
