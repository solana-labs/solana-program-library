//! Vault management instructions.

use {
    crate::pack::check_data_len,
    arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs},
    num_enum::TryFromPrimitive,
    solana_program::program_error::ProgramError,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VaultInstruction {
    /// Initialize on-chain records for a new user
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    UserInit,

    /// Add liquidity to the Vault
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    AddLiquidity {
        max_token_a_amount: u64,
        max_token_b_amount: u64,
    },

    /// Lock liquidity in the Vault
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    LockLiquidity { amount: u64 },

    /// Unlock liquidity in the Vault
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    UnlockLiquidity { amount: u64 },

    /// Remove liquidity from the Vault
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    RemoveLiquidity { amount: u64 },

    /// Initialize multisig with a new set of admin signatures
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    SetAdminSigners { min_signatures: u8 },

    /// Remove Fund specific multisig, Main Router's default auth will be used
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    RemoveMultisig,

    /// Set minimum crank interval for the Vault
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    SetMinCrankInterval { min_crank_interval: u32 },

    /// Set fee for the Vault
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    SetFee { fee: f32 },

    /// Set underlying protocol fee for the Vault
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    SetExternalFee { external_fee: f32 },

    /// Disable new deposits to the Vault
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    DisableDeposits,

    /// Allow new deposits to the Vault
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    EnableDeposits,

    /// Disable withdrawals from the Vault
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    DisableWithdrawals,

    /// Allow withdrawals from the Vault
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    EnableWithdrawals,

    /// Run crank operation on the Vault
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    Crank { step: u64 },

    /// Initialize the Vault
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    Init { step: u64 },

    /// Shutdown the Vault
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    Shutdown,

    /// Withdraw collected fees
    /// # Account references are strategy specific,
    ///   see particular Vault instructions handlers for more info
    WithdrawFees { amount: u64 },
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum VaultInstructionType {
    UserInit,
    AddLiquidity,
    LockLiquidity,
    UnlockLiquidity,
    RemoveLiquidity,
    SetAdminSigners,
    RemoveMultisig,
    SetMinCrankInterval,
    SetFee,
    SetExternalFee,
    DisableDeposits,
    EnableDeposits,
    DisableWithdrawals,
    EnableWithdrawals,
    Crank,
    Init,
    Shutdown,
    WithdrawFees,
}

impl VaultInstruction {
    pub const MAX_LEN: usize = 17;
    pub const USER_INIT_LEN: usize = 1;
    pub const ADD_LIQUIDITY_LEN: usize = 17;
    pub const LOCK_LIQUIDITY_LEN: usize = 9;
    pub const UNLOCK_LIQUIDITY_LEN: usize = 9;
    pub const REMOVE_LIQUIDITY_LEN: usize = 9;
    pub const SET_ADMIN_SIGNERS_LEN: usize = 2;
    pub const REMOVE_MULTISIG_LEN: usize = 1;
    pub const SET_MIN_CRANK_INTERVAL_LEN: usize = 5;
    pub const SET_FEE_LEN: usize = 5;
    pub const SET_EXTERNAL_FEE_LEN: usize = 5;
    pub const DISABLE_DEPOSITS_LEN: usize = 1;
    pub const ENABLE_DEPOSITS_LEN: usize = 1;
    pub const DISABLE_WITHDRAWALS_LEN: usize = 1;
    pub const ENABLE_WITHDRAWALS_LEN: usize = 1;
    pub const CRANK_LEN: usize = 9;
    pub const INIT_LEN: usize = 9;
    pub const SHUTDOWN_LEN: usize = 1;
    pub const WITHDRAW_FEES_LEN: usize = 9;

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        match self {
            Self::UserInit { .. } => self.pack_user_init(output),
            Self::AddLiquidity { .. } => self.pack_add_liquidity(output),
            Self::RemoveLiquidity { .. } => self.pack_remove_liquidity(output),
            Self::LockLiquidity { .. } => self.pack_lock_liquidity(output),
            Self::UnlockLiquidity { .. } => self.pack_unlock_liquidity(output),
            Self::SetAdminSigners { .. } => self.pack_set_admin_signers(output),
            Self::RemoveMultisig { .. } => self.pack_remove_multisig(output),
            Self::SetMinCrankInterval { .. } => self.pack_set_min_crank_interval(output),
            Self::SetFee { .. } => self.pack_set_fee(output),
            Self::SetExternalFee { .. } => self.pack_set_external_fee(output),
            Self::DisableDeposits { .. } => self.pack_disable_deposits(output),
            Self::EnableDeposits { .. } => self.pack_enable_deposits(output),
            Self::DisableWithdrawals { .. } => self.pack_disable_withdrawals(output),
            Self::EnableWithdrawals { .. } => self.pack_enable_withdrawals(output),
            Self::Crank { .. } => self.pack_crank(output),
            Self::Init { .. } => self.pack_init(output),
            Self::Shutdown { .. } => self.pack_shutdown(output),
            Self::WithdrawFees { .. } => self.pack_withdraw_fees(output),
        }
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; VaultInstruction::MAX_LEN] = [0; VaultInstruction::MAX_LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    pub fn unpack(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, 1)?;
        let instruction_type = VaultInstructionType::try_from_primitive(input[0])
            .or(Err(ProgramError::InvalidInstructionData))?;
        match instruction_type {
            VaultInstructionType::UserInit => VaultInstruction::unpack_user_init(input),
            VaultInstructionType::AddLiquidity => VaultInstruction::unpack_add_liquidity(input),
            VaultInstructionType::LockLiquidity => VaultInstruction::unpack_lock_liquidity(input),
            VaultInstructionType::UnlockLiquidity => {
                VaultInstruction::unpack_unlock_liquidity(input)
            }
            VaultInstructionType::RemoveLiquidity => {
                VaultInstruction::unpack_remove_liquidity(input)
            }
            VaultInstructionType::SetAdminSigners => {
                VaultInstruction::unpack_set_admin_signers(input)
            }
            VaultInstructionType::RemoveMultisig => VaultInstruction::unpack_remove_multisig(input),
            VaultInstructionType::SetMinCrankInterval => {
                VaultInstruction::unpack_set_min_crank_interval(input)
            }
            VaultInstructionType::SetFee => VaultInstruction::unpack_set_fee(input),
            VaultInstructionType::SetExternalFee => {
                VaultInstruction::unpack_set_external_fee(input)
            }
            VaultInstructionType::DisableDeposits => {
                VaultInstruction::unpack_disable_deposits(input)
            }
            VaultInstructionType::EnableDeposits => VaultInstruction::unpack_enable_deposits(input),
            VaultInstructionType::DisableWithdrawals => {
                VaultInstruction::unpack_disable_withdrawals(input)
            }
            VaultInstructionType::EnableWithdrawals => {
                VaultInstruction::unpack_enable_withdrawals(input)
            }
            VaultInstructionType::Crank => VaultInstruction::unpack_crank(input),
            VaultInstructionType::Init => VaultInstruction::unpack_init(input),
            VaultInstructionType::Shutdown => VaultInstruction::unpack_shutdown(input),
            VaultInstructionType::WithdrawFees => VaultInstruction::unpack_withdraw_fees(input),
        }
    }

    fn pack_user_init(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::USER_INIT_LEN)?;

        if let VaultInstruction::UserInit = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = VaultInstructionType::UserInit as u8;

            Ok(VaultInstruction::USER_INIT_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_add_liquidity(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::ADD_LIQUIDITY_LEN)?;

        if let VaultInstruction::AddLiquidity {
            max_token_a_amount,
            max_token_b_amount,
        } = self
        {
            let output = array_mut_ref![output, 0, VaultInstruction::ADD_LIQUIDITY_LEN];
            let (instruction_type_out, max_token_a_amount_out, max_token_b_amount_out) =
                mut_array_refs![output, 1, 8, 8];

            instruction_type_out[0] = VaultInstructionType::AddLiquidity as u8;

            *max_token_a_amount_out = max_token_a_amount.to_le_bytes();
            *max_token_b_amount_out = max_token_b_amount.to_le_bytes();

            Ok(VaultInstruction::ADD_LIQUIDITY_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_lock_liquidity(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::LOCK_LIQUIDITY_LEN)?;

        if let VaultInstruction::LockLiquidity { amount } = self {
            let output = array_mut_ref![output, 0, VaultInstruction::LOCK_LIQUIDITY_LEN];
            let (instruction_type_out, amount_out) = mut_array_refs![output, 1, 8];

            instruction_type_out[0] = VaultInstructionType::LockLiquidity as u8;

            *amount_out = amount.to_le_bytes();

            Ok(VaultInstruction::LOCK_LIQUIDITY_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_unlock_liquidity(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::UNLOCK_LIQUIDITY_LEN)?;

        if let VaultInstruction::UnlockLiquidity { amount } = self {
            let output = array_mut_ref![output, 0, VaultInstruction::UNLOCK_LIQUIDITY_LEN];
            let (instruction_type_out, amount_out) = mut_array_refs![output, 1, 8];

            instruction_type_out[0] = VaultInstructionType::UnlockLiquidity as u8;

            *amount_out = amount.to_le_bytes();

            Ok(VaultInstruction::UNLOCK_LIQUIDITY_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_remove_liquidity(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::REMOVE_LIQUIDITY_LEN)?;

        if let VaultInstruction::RemoveLiquidity { amount } = self {
            let output = array_mut_ref![output, 0, VaultInstruction::REMOVE_LIQUIDITY_LEN];
            let (instruction_type_out, amount_out) = mut_array_refs![output, 1, 8];

            instruction_type_out[0] = VaultInstructionType::RemoveLiquidity as u8;

            *amount_out = amount.to_le_bytes();

            Ok(VaultInstruction::REMOVE_LIQUIDITY_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_set_admin_signers(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::SET_ADMIN_SIGNERS_LEN)?;

        if let VaultInstruction::SetAdminSigners { min_signatures } = self {
            let output = array_mut_ref![output, 0, VaultInstruction::SET_ADMIN_SIGNERS_LEN];
            let (instruction_type_out, min_signatures_out) = mut_array_refs![output, 1, 1];

            instruction_type_out[0] = VaultInstructionType::SetAdminSigners as u8;
            min_signatures_out[0] = *min_signatures;

            Ok(VaultInstruction::SET_ADMIN_SIGNERS_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_remove_multisig(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::REMOVE_MULTISIG_LEN)?;

        if let VaultInstruction::RemoveMultisig = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = VaultInstructionType::RemoveMultisig as u8;

            Ok(VaultInstruction::REMOVE_MULTISIG_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_set_min_crank_interval(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::SET_MIN_CRANK_INTERVAL_LEN)?;

        if let VaultInstruction::SetMinCrankInterval { min_crank_interval } = self {
            let output = array_mut_ref![output, 0, VaultInstruction::SET_MIN_CRANK_INTERVAL_LEN];
            let (instruction_type_out, min_crank_interval_out) = mut_array_refs![output, 1, 4];

            instruction_type_out[0] = VaultInstructionType::SetMinCrankInterval as u8;

            *min_crank_interval_out = min_crank_interval.to_le_bytes();

            Ok(VaultInstruction::SET_MIN_CRANK_INTERVAL_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_set_fee(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::SET_FEE_LEN)?;

        if let VaultInstruction::SetFee { fee } = self {
            let output = array_mut_ref![output, 0, VaultInstruction::SET_FEE_LEN];
            let (instruction_type_out, fee_out) = mut_array_refs![output, 1, 4];

            instruction_type_out[0] = VaultInstructionType::SetFee as u8;

            *fee_out = fee.to_le_bytes();

            Ok(VaultInstruction::SET_FEE_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_set_external_fee(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::SET_EXTERNAL_FEE_LEN)?;

        if let VaultInstruction::SetExternalFee { external_fee } = self {
            let output = array_mut_ref![output, 0, VaultInstruction::SET_EXTERNAL_FEE_LEN];
            let (instruction_type_out, external_fee_out) = mut_array_refs![output, 1, 4];

            instruction_type_out[0] = VaultInstructionType::SetExternalFee as u8;

            *external_fee_out = external_fee.to_le_bytes();

            Ok(VaultInstruction::SET_EXTERNAL_FEE_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_disable_deposits(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::DISABLE_DEPOSITS_LEN)?;

        if let VaultInstruction::DisableDeposits = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = VaultInstructionType::DisableDeposits as u8;

            Ok(VaultInstruction::DISABLE_DEPOSITS_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_enable_deposits(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::ENABLE_DEPOSITS_LEN)?;

        if let VaultInstruction::EnableDeposits = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = VaultInstructionType::EnableDeposits as u8;

            Ok(VaultInstruction::ENABLE_DEPOSITS_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_disable_withdrawals(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::DISABLE_WITHDRAWALS_LEN)?;

        if let VaultInstruction::DisableWithdrawals = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = VaultInstructionType::DisableWithdrawals as u8;

            Ok(VaultInstruction::DISABLE_WITHDRAWALS_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_enable_withdrawals(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::ENABLE_WITHDRAWALS_LEN)?;

        if let VaultInstruction::EnableWithdrawals = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = VaultInstructionType::EnableWithdrawals as u8;

            Ok(VaultInstruction::ENABLE_WITHDRAWALS_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_crank(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::CRANK_LEN)?;

        if let VaultInstruction::Crank { step } = self {
            let output = array_mut_ref![output, 0, VaultInstruction::CRANK_LEN];
            let (instruction_type_out, step_out) = mut_array_refs![output, 1, 8];

            instruction_type_out[0] = VaultInstructionType::Crank as u8;

            *step_out = step.to_le_bytes();

            Ok(VaultInstruction::CRANK_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_init(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::INIT_LEN)?;

        if let VaultInstruction::Init { step } = self {
            let output = array_mut_ref![output, 0, VaultInstruction::INIT_LEN];
            let (instruction_type_out, step_out) = mut_array_refs![output, 1, 8];

            instruction_type_out[0] = VaultInstructionType::Init as u8;

            *step_out = step.to_le_bytes();

            Ok(VaultInstruction::INIT_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_shutdown(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::SHUTDOWN_LEN)?;

        if let VaultInstruction::Shutdown = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = VaultInstructionType::Shutdown as u8;

            Ok(VaultInstruction::SHUTDOWN_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_withdraw_fees(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, VaultInstruction::WITHDRAW_FEES_LEN)?;

        if let VaultInstruction::WithdrawFees { amount } = self {
            let output = array_mut_ref![output, 0, VaultInstruction::WITHDRAW_FEES_LEN];
            let (instruction_type_out, amount_out) = mut_array_refs![output, 1, 8];

            instruction_type_out[0] = VaultInstructionType::WithdrawFees as u8;

            *amount_out = amount.to_le_bytes();

            Ok(VaultInstruction::WITHDRAW_FEES_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn unpack_user_init(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::USER_INIT_LEN)?;
        Ok(Self::UserInit)
    }

    fn unpack_add_liquidity(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::ADD_LIQUIDITY_LEN)?;

        let input = array_ref![input, 1, VaultInstruction::ADD_LIQUIDITY_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (max_token_a_amount, max_token_b_amount) = array_refs![input, 8, 8];

        Ok(Self::AddLiquidity {
            max_token_a_amount: u64::from_le_bytes(*max_token_a_amount),
            max_token_b_amount: u64::from_le_bytes(*max_token_b_amount),
        })
    }

    fn unpack_lock_liquidity(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::LOCK_LIQUIDITY_LEN)?;
        Ok(Self::LockLiquidity {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_unlock_liquidity(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::UNLOCK_LIQUIDITY_LEN)?;
        Ok(Self::UnlockLiquidity {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_remove_liquidity(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::REMOVE_LIQUIDITY_LEN)?;
        Ok(Self::RemoveLiquidity {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_set_admin_signers(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::SET_ADMIN_SIGNERS_LEN)?;

        let input = array_ref![input, 1, VaultInstruction::SET_ADMIN_SIGNERS_LEN - 1];

        Ok(Self::SetAdminSigners {
            min_signatures: input[0],
        })
    }

    fn unpack_remove_multisig(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::REMOVE_MULTISIG_LEN)?;
        Ok(Self::RemoveMultisig)
    }

    fn unpack_set_min_crank_interval(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::SET_MIN_CRANK_INTERVAL_LEN)?;
        Ok(Self::SetMinCrankInterval {
            min_crank_interval: u32::from_le_bytes(*array_ref![input, 1, 4]),
        })
    }

    fn unpack_set_fee(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::SET_FEE_LEN)?;
        Ok(Self::SetFee {
            fee: f32::from_le_bytes(*array_ref![input, 1, 4]),
        })
    }

    fn unpack_set_external_fee(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::SET_EXTERNAL_FEE_LEN)?;
        Ok(Self::SetExternalFee {
            external_fee: f32::from_le_bytes(*array_ref![input, 1, 4]),
        })
    }

    fn unpack_disable_deposits(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::DISABLE_DEPOSITS_LEN)?;
        Ok(Self::DisableDeposits)
    }

    fn unpack_enable_deposits(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::ENABLE_DEPOSITS_LEN)?;
        Ok(Self::EnableDeposits)
    }

    fn unpack_disable_withdrawals(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::DISABLE_WITHDRAWALS_LEN)?;
        Ok(Self::DisableWithdrawals)
    }

    fn unpack_enable_withdrawals(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::ENABLE_WITHDRAWALS_LEN)?;
        Ok(Self::EnableWithdrawals)
    }

    fn unpack_crank(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::CRANK_LEN)?;
        Ok(Self::Crank {
            step: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_init(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::INIT_LEN)?;
        Ok(Self::Init {
            step: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_shutdown(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::SHUTDOWN_LEN)?;
        Ok(Self::Shutdown)
    }

    fn unpack_withdraw_fees(input: &[u8]) -> Result<VaultInstruction, ProgramError> {
        check_data_len(input, VaultInstruction::WITHDRAW_FEES_LEN)?;
        Ok(Self::WithdrawFees {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }
}

impl std::fmt::Display for VaultInstructionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            VaultInstructionType::UserInit => write!(f, "UserInit"),
            VaultInstructionType::AddLiquidity => write!(f, "AddLiquidity"),
            VaultInstructionType::LockLiquidity => write!(f, "LockLiquidity"),
            VaultInstructionType::UnlockLiquidity => write!(f, "UnlockLiquidity"),
            VaultInstructionType::RemoveLiquidity => write!(f, "RemoveLiquidity"),
            VaultInstructionType::SetAdminSigners => write!(f, "SetAdminSigners"),
            VaultInstructionType::RemoveMultisig => write!(f, "RemoveMultisig"),
            VaultInstructionType::SetMinCrankInterval => write!(f, "SetMinCrankInterval"),
            VaultInstructionType::SetFee => write!(f, "SetFee"),
            VaultInstructionType::SetExternalFee => write!(f, "SetExternalFee"),
            VaultInstructionType::DisableDeposits => write!(f, "DisableDeposits"),
            VaultInstructionType::EnableDeposits => write!(f, "EnableDeposits"),
            VaultInstructionType::DisableWithdrawals => write!(f, "DisableWithdrawals"),
            VaultInstructionType::EnableWithdrawals => write!(f, "EnableWithdrawals"),
            VaultInstructionType::Crank => write!(f, "Crank"),
            VaultInstructionType::Init => write!(f, "Init"),
            VaultInstructionType::Shutdown => write!(f, "Shutdown"),
            VaultInstructionType::WithdrawFees => write!(f, "WithdrawFees"),
        }
    }
}
