//! Raydium router instructions.

use {
    crate::pack::check_data_len,
    arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs},
    num_enum::TryFromPrimitive,
    solana_program::program_error::ProgramError,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum AmmInstruction {
    /// Initialize on-chain records for a new user
    /// # Account references are protocol specific,
    ///   see particular Router instructions handlers for more info
    UserInit,

    /// Add liquidity to the AMM Pool
    /// # Account references are protocol specific,
    ///   see particular Router instructions handlers for more info
    AddLiquidity {
        max_token_a_amount: u64,
        max_token_b_amount: u64,
    },

    /// Remove liquidity from the AMM Pool
    /// # Account references are protocol specific,
    ///   see particular Router instructions handlers for more info
    RemoveLiquidity { amount: u64 },

    /// Swap tokens in the AMM Pool
    /// # Account references are protocol specific,
    ///   see particular Router instructions handlers for more info
    Swap {
        token_a_amount_in: u64,
        token_b_amount_in: u64,
        min_token_amount_out: u64,
    },

    /// Stake LP tokens to the Farm
    /// # Account references are protocol specific,
    ///   see particular Router instructions handlers for more info
    Stake { amount: u64 },

    /// Unstake LP tokens from the Farm
    /// # Account references are protocol specific,
    ///   see particular Router instructions handlers for more info
    Unstake { amount: u64 },

    /// Claim pending rewards from the Farm
    /// # Account references are protocol specific,
    ///   see particular Router instructions handlers for more info
    Harvest,

    /// Wrap the token to protocol specific token
    /// # Account references are protocol specific,
    ///   see particular Router instructions handlers for more info
    WrapToken { amount: u64 },

    /// Unwrap the token from protocol specific token
    /// # Account references are protocol specific,
    ///   see particular Router instructions handlers for more info
    UnwrapToken { amount: u64 },
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum AmmInstructionType {
    UserInit,
    AddLiquidity,
    RemoveLiquidity,
    Swap,
    Stake,
    Unstake,
    Harvest,
    WrapToken,
    UnwrapToken,
}

impl AmmInstruction {
    pub const MAX_LEN: usize = 25;
    pub const USER_INIT_LEN: usize = 1;
    pub const ADD_LIQUIDITY_LEN: usize = 17;
    pub const REMOVE_LIQUIDITY_LEN: usize = 9;
    pub const SWAP_LEN: usize = 25;
    pub const STAKE_LEN: usize = 9;
    pub const UNSTAKE_LEN: usize = 9;
    pub const HARVEST_LEN: usize = 1;
    pub const WRAP_TOKEN_LEN: usize = 9;
    pub const UNWRAP_TOKEN_LEN: usize = 9;

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        match self {
            Self::UserInit { .. } => self.pack_user_init(output),
            Self::AddLiquidity { .. } => self.pack_add_liquidity(output),
            Self::RemoveLiquidity { .. } => self.pack_remove_liquidity(output),
            Self::Swap { .. } => self.pack_swap(output),
            Self::Stake { .. } => self.pack_stake(output),
            Self::Unstake { .. } => self.pack_unstake(output),
            Self::Harvest { .. } => self.pack_harvest(output),
            Self::WrapToken { .. } => self.pack_wrap_token(output),
            Self::UnwrapToken { .. } => self.pack_unwrap_token(output),
        }
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; AmmInstruction::MAX_LEN] = [0; AmmInstruction::MAX_LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    pub fn unpack(input: &[u8]) -> Result<AmmInstruction, ProgramError> {
        check_data_len(input, 1)?;
        let instruction_type = AmmInstructionType::try_from_primitive(input[0])
            .or(Err(ProgramError::InvalidInstructionData))?;
        match instruction_type {
            AmmInstructionType::UserInit => AmmInstruction::unpack_user_init(input),
            AmmInstructionType::AddLiquidity => AmmInstruction::unpack_add_liquidity(input),
            AmmInstructionType::RemoveLiquidity => AmmInstruction::unpack_remove_liquidity(input),
            AmmInstructionType::Swap => AmmInstruction::unpack_swap(input),
            AmmInstructionType::Stake => AmmInstruction::unpack_stake(input),
            AmmInstructionType::Unstake => AmmInstruction::unpack_unstake(input),
            AmmInstructionType::Harvest => AmmInstruction::unpack_harvest(input),
            AmmInstructionType::WrapToken => AmmInstruction::unpack_wrap_token(input),
            AmmInstructionType::UnwrapToken => AmmInstruction::unpack_unwrap_token(input),
        }
    }

    fn pack_user_init(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, AmmInstruction::USER_INIT_LEN)?;

        if let AmmInstruction::UserInit = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = AmmInstructionType::UserInit as u8;

            Ok(AmmInstruction::USER_INIT_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_add_liquidity(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, AmmInstruction::ADD_LIQUIDITY_LEN)?;

        if let AmmInstruction::AddLiquidity {
            max_token_a_amount,
            max_token_b_amount,
        } = self
        {
            let output = array_mut_ref![output, 0, AmmInstruction::ADD_LIQUIDITY_LEN];
            let (instruction_type_pack, max_token_a_amount_pack, max_token_b_amount_pack) =
                mut_array_refs![output, 1, 8, 8];

            instruction_type_pack[0] = AmmInstructionType::AddLiquidity as u8;

            *max_token_a_amount_pack = max_token_a_amount.to_le_bytes();
            *max_token_b_amount_pack = max_token_b_amount.to_le_bytes();

            Ok(AmmInstruction::ADD_LIQUIDITY_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_remove_liquidity(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, AmmInstruction::REMOVE_LIQUIDITY_LEN)?;

        if let AmmInstruction::RemoveLiquidity { amount } = self {
            let output = array_mut_ref![output, 0, AmmInstruction::REMOVE_LIQUIDITY_LEN];
            let (instruction_type_pack, amount_pack) = mut_array_refs![output, 1, 8];

            instruction_type_pack[0] = AmmInstructionType::RemoveLiquidity as u8;

            *amount_pack = amount.to_le_bytes();

            Ok(AmmInstruction::REMOVE_LIQUIDITY_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_swap(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, AmmInstruction::SWAP_LEN)?;

        if let AmmInstruction::Swap {
            token_a_amount_in,
            token_b_amount_in,
            min_token_amount_out,
        } = self
        {
            let output = array_mut_ref![output, 0, AmmInstruction::SWAP_LEN];
            let (
                instruction_type_pack,
                token_a_amount_in_pack,
                token_b_amount_in_pack,
                min_token_amount_out_pack,
            ) = mut_array_refs![output, 1, 8, 8, 8];

            instruction_type_pack[0] = AmmInstructionType::Swap as u8;

            *token_a_amount_in_pack = token_a_amount_in.to_le_bytes();
            *token_b_amount_in_pack = token_b_amount_in.to_le_bytes();
            *min_token_amount_out_pack = min_token_amount_out.to_le_bytes();

            Ok(AmmInstruction::SWAP_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_stake(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, AmmInstruction::STAKE_LEN)?;

        if let AmmInstruction::Stake { amount } = self {
            let output = array_mut_ref![output, 0, AmmInstruction::STAKE_LEN];
            let (instruction_type_pack, amount_pack) = mut_array_refs![output, 1, 8];

            instruction_type_pack[0] = AmmInstructionType::Stake as u8;

            *amount_pack = amount.to_le_bytes();

            Ok(AmmInstruction::STAKE_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_unstake(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, AmmInstruction::UNSTAKE_LEN)?;

        if let AmmInstruction::Unstake { amount } = self {
            let output = array_mut_ref![output, 0, AmmInstruction::UNSTAKE_LEN];
            let (instruction_type_pack, amount_pack) = mut_array_refs![output, 1, 8];

            instruction_type_pack[0] = AmmInstructionType::Unstake as u8;

            *amount_pack = amount.to_le_bytes();

            Ok(AmmInstruction::UNSTAKE_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_harvest(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, AmmInstruction::HARVEST_LEN)?;

        if let AmmInstruction::Harvest = self {
            let instruction_type_pack = array_mut_ref![output, 0, 1];

            instruction_type_pack[0] = AmmInstructionType::Harvest as u8;

            Ok(AmmInstruction::HARVEST_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_wrap_token(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, AmmInstruction::WRAP_TOKEN_LEN)?;

        if let AmmInstruction::WrapToken { amount } = self {
            let output = array_mut_ref![output, 0, AmmInstruction::WRAP_TOKEN_LEN];
            let (instruction_type_pack, amount_pack) = mut_array_refs![output, 1, 8];

            instruction_type_pack[0] = AmmInstructionType::WrapToken as u8;

            *amount_pack = amount.to_le_bytes();

            Ok(AmmInstruction::WRAP_TOKEN_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_unwrap_token(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, AmmInstruction::UNWRAP_TOKEN_LEN)?;

        if let AmmInstruction::UnwrapToken { amount } = self {
            let output = array_mut_ref![output, 0, AmmInstruction::UNWRAP_TOKEN_LEN];
            let (instruction_type_pack, amount_pack) = mut_array_refs![output, 1, 8];

            instruction_type_pack[0] = AmmInstructionType::UnwrapToken as u8;

            *amount_pack = amount.to_le_bytes();

            Ok(AmmInstruction::UNWRAP_TOKEN_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn unpack_user_init(input: &[u8]) -> Result<AmmInstruction, ProgramError> {
        check_data_len(input, AmmInstruction::USER_INIT_LEN)?;
        Ok(Self::UserInit)
    }

    fn unpack_add_liquidity(input: &[u8]) -> Result<AmmInstruction, ProgramError> {
        check_data_len(input, AmmInstruction::ADD_LIQUIDITY_LEN)?;

        let input = array_ref![input, 1, AmmInstruction::ADD_LIQUIDITY_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (max_token_a_amount, max_token_b_amount) = array_refs![input, 8, 8];

        Ok(Self::AddLiquidity {
            max_token_a_amount: u64::from_le_bytes(*max_token_a_amount),
            max_token_b_amount: u64::from_le_bytes(*max_token_b_amount),
        })
    }

    fn unpack_remove_liquidity(input: &[u8]) -> Result<AmmInstruction, ProgramError> {
        check_data_len(input, AmmInstruction::REMOVE_LIQUIDITY_LEN)?;
        Ok(Self::RemoveLiquidity {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_swap(input: &[u8]) -> Result<AmmInstruction, ProgramError> {
        check_data_len(input, AmmInstruction::SWAP_LEN)?;

        let input = array_ref![input, 1, AmmInstruction::SWAP_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (token_a_amount_in, token_b_amount_in, min_token_amount_out) =
            array_refs![input, 8, 8, 8];

        Ok(Self::Swap {
            token_a_amount_in: u64::from_le_bytes(*token_a_amount_in),
            token_b_amount_in: u64::from_le_bytes(*token_b_amount_in),
            min_token_amount_out: u64::from_le_bytes(*min_token_amount_out),
        })
    }

    fn unpack_stake(input: &[u8]) -> Result<AmmInstruction, ProgramError> {
        check_data_len(input, AmmInstruction::STAKE_LEN)?;
        Ok(Self::Stake {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_unstake(input: &[u8]) -> Result<AmmInstruction, ProgramError> {
        check_data_len(input, AmmInstruction::UNSTAKE_LEN)?;
        Ok(Self::Unstake {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_harvest(input: &[u8]) -> Result<AmmInstruction, ProgramError> {
        check_data_len(input, AmmInstruction::HARVEST_LEN)?;
        Ok(Self::Harvest)
    }

    fn unpack_wrap_token(input: &[u8]) -> Result<AmmInstruction, ProgramError> {
        check_data_len(input, AmmInstruction::WRAP_TOKEN_LEN)?;
        Ok(Self::WrapToken {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_unwrap_token(input: &[u8]) -> Result<AmmInstruction, ProgramError> {
        check_data_len(input, AmmInstruction::UNWRAP_TOKEN_LEN)?;
        Ok(Self::UnwrapToken {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }
}

impl std::fmt::Display for AmmInstructionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            AmmInstructionType::UserInit => write!(f, "UserInit"),
            AmmInstructionType::AddLiquidity => write!(f, "AddLiquidity"),
            AmmInstructionType::RemoveLiquidity => write!(f, "RemoveLiquidity"),
            AmmInstructionType::Swap => write!(f, "Swap"),
            AmmInstructionType::Stake => write!(f, "Stake"),
            AmmInstructionType::Unstake => write!(f, "Unstake"),
            AmmInstructionType::Harvest => write!(f, "Harvest"),
            AmmInstructionType::WrapToken => write!(f, "WrapToken"),
            AmmInstructionType::UnwrapToken => write!(f, "UnwrapToken"),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_vec_serialization() {
        let ri1 = super::AmmInstruction::AddLiquidity {
            max_token_a_amount: 100,
            max_token_b_amount: 200,
        };

        let vec = ri1.to_vec().unwrap();

        let ri2 = super::AmmInstruction::unpack(&vec[..]).unwrap();

        assert_eq!(ri1, ri2);
    }

    #[test]
    fn test_slice_serialization() {
        let ri1 = super::AmmInstruction::AddLiquidity {
            max_token_a_amount: 100,
            max_token_b_amount: 200,
        };

        let mut output: [u8; super::AmmInstruction::ADD_LIQUIDITY_LEN] =
            [0; super::AmmInstruction::ADD_LIQUIDITY_LEN];
        ri1.pack(&mut output[..]).unwrap();

        let ri2 = super::AmmInstruction::unpack(&output).unwrap();

        assert_eq!(ri1, ri2);
    }
}
