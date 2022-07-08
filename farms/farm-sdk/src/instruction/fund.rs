//! Fund management instructions.

use {
    crate::{
        fund::{FundAssetsTrackingConfig, FundCustodyType, FundSchedule, FundVaultType},
        instruction::{amm::AmmInstruction, vault::VaultInstruction},
        pack::{
            check_data_len, pack_array_string64, pack_bool, unpack_array_string64, unpack_bool,
        },
        string::ArrayString64,
    },
    arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs},
    num_enum::TryFromPrimitive,
    solana_program::program_error::ProgramError,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FundInstruction {
    /// Initialize on-chain record for a new user
    UserInit,

    /// Request deposit to the Fund
    RequestDeposit { amount: u64 },

    /// Cancel pending deposit to the Fund
    CancelDeposit,

    /// Request withdrawal from the Fund
    RequestWithdrawal { amount: u64 },

    /// Cancel pending withdrawal from the Fund
    CancelWithdrawal,

    /// Initialize the Fund
    Init { step: u64 },

    /// Set schedule and enable deposits
    SetDepositSchedule { schedule: FundSchedule },

    /// Disable all deposits
    DisableDeposits,

    /// Approve pending deposit for the user
    ApproveDeposit { amount: u64 },

    /// Deny pending deposit for the user
    DenyDeposit { deny_reason: ArrayString64 },

    /// Set schedule and enable withdrawals
    SetWithdrawalSchedule { schedule: FundSchedule },

    /// Disable all withdrawals
    DisableWithdrawals,

    /// Approve pending withdrawal for the user
    ApproveWithdrawal { amount: u64 },

    /// Deny pending withdrawal for the user
    DenyWithdrawal { deny_reason: ArrayString64 },

    /// Move funds from deposit/withdrawal custody to trading custody
    LockAssets { amount: u64 },

    /// Move funds from trading custody to deposit/withdrawal custody
    UnlockAssets { amount: u64 },

    /// Initialize multisig with a new set of admin signatures
    SetAdminSigners { min_signatures: u8 },

    /// Remove Fund specific multisig, Main Router's default auth will be used
    RemoveMultisig,

    /// Set parameters for assets tracking
    SetAssetsTrackingConfig { config: FundAssetsTrackingConfig },

    /// Update Fund assets with the Vault's holdings
    UpdateAssetsWithVault,

    /// Update Fund assets with the Custody's holdings
    UpdateAssetsWithCustody,

    /// Add a Vault to the Fund
    AddVault {
        target_hash: u64,
        vault_id: u32,
        vault_type: FundVaultType,
    },

    /// Remove a Vault from the Fund
    RemoveVault {
        target_hash: u64,
        vault_type: FundVaultType,
    },

    /// Add a Custody to the Fund
    AddCustody {
        target_hash: u64,
        custody_id: u32,
        custody_type: FundCustodyType,
    },

    /// Remove a Custody from the Fund
    RemoveCustody {
        target_hash: u64,
        custody_type: FundCustodyType,
    },

    /// Start Fund liquidation process
    StartLiquidation,

    /// Stop Fund liquidation process
    StopLiquidation,

    /// Withdraw collected fees from the Fund
    WithdrawFees { amount: u64 },

    /// Raydium pool instructions
    AmmInstructionRaydium { instruction: AmmInstruction },

    /// Raydium vault instructions
    VaultInstructionRaydium { instruction: VaultInstruction },

    /// Orca pool instructions
    AmmInstructionOrca { instruction: AmmInstruction },

    /// Orca vault instructions
    VaultInstructionOrca { instruction: VaultInstruction },
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum FundInstructionType {
    UserInit,
    RequestDeposit,
    CancelDeposit,
    RequestWithdrawal,
    CancelWithdrawal,
    Init,
    SetDepositSchedule,
    DisableDeposits,
    ApproveDeposit,
    DenyDeposit,
    SetWithdrawalSchedule,
    DisableWithdrawals,
    ApproveWithdrawal,
    DenyWithdrawal,
    LockAssets,
    UnlockAssets,
    SetAdminSigners,
    RemoveMultisig,
    SetAssetsTrackingConfig,
    UpdateAssetsWithVault,
    UpdateAssetsWithCustody,
    AddVault,
    RemoveVault,
    AddCustody,
    RemoveCustody,
    StartLiquidation,
    StopLiquidation,
    WithdrawFees,
    AmmInstructionRaydium,
    VaultInstructionRaydium,
    AmmInstructionOrca,
    VaultInstructionOrca,
}

impl FundInstruction {
    pub const MAX_LEN: usize = 65;
    pub const USER_INIT_LEN: usize = 1;
    pub const REQUEST_DEPOSIT_LEN: usize = 9;
    pub const CANCEL_DEPOSIT_LEN: usize = 1;
    pub const REQUEST_WITHDRAWAL_LEN: usize = 9;
    pub const CANCEL_WITHDRAWAL_LEN: usize = 1;
    pub const INIT_LEN: usize = 9;
    pub const SET_DEPOSIT_SCHEDULE_LEN: usize = 42;
    pub const DISABLE_DEPOSITS_LEN: usize = 1;
    pub const APPROVE_DEPOSIT_LEN: usize = 9;
    pub const DENY_DEPOSIT_LEN: usize = 65;
    pub const SET_WITHDRAWAL_SCHEDULE_LEN: usize = 42;
    pub const DISABLE_WITHDRAWALS_LEN: usize = 1;
    pub const APPROVE_WITHDRAWAL_LEN: usize = 9;
    pub const DENY_WITHDRAWAL_LEN: usize = 65;
    pub const LOCK_ASSETS_LEN: usize = 9;
    pub const UNLOCK_ASSETS_LEN: usize = 9;
    pub const SET_ADMIN_SIGNERS_LEN: usize = 2;
    pub const REMOVE_MULTISIG_LEN: usize = 1;
    pub const SET_ASSETS_TRACKING_CONFIG_LEN: usize = 34;
    pub const UPDATE_ASSETS_WITH_VAULT_LEN: usize = 1;
    pub const UPDATE_ASSETS_WITH_CUSTODY_LEN: usize = 1;
    pub const ADD_VAULT_LEN: usize = 14;
    pub const REMOVE_VAULT_LEN: usize = 10;
    pub const ADD_CUSTODY_LEN: usize = 14;
    pub const REMOVE_CUSTODY_LEN: usize = 10;
    pub const START_LIQUIDATION_LEN: usize = 1;
    pub const STOP_LIQUIDATION_LEN: usize = 1;
    pub const WITHDRAW_FEES_LEN: usize = 9;

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        match self {
            Self::UserInit { .. } => self.pack_user_init(output),
            Self::RequestDeposit { .. } => self.pack_request_deposit(output),
            Self::CancelDeposit { .. } => self.pack_cancel_deposit(output),
            Self::RequestWithdrawal { .. } => self.pack_request_withdrawal(output),
            Self::CancelWithdrawal { .. } => self.pack_cancel_withdrawal(output),
            Self::Init { .. } => self.pack_init(output),
            Self::SetDepositSchedule { .. } => self.pack_set_deposit_schedule(output),
            Self::DisableDeposits { .. } => self.pack_disable_deposits(output),
            Self::ApproveDeposit { .. } => self.pack_approve_deposit(output),
            Self::DenyDeposit { .. } => self.pack_deny_deposit(output),
            Self::SetWithdrawalSchedule { .. } => self.pack_set_withdrawal_schedule(output),
            Self::DisableWithdrawals { .. } => self.pack_disable_withdrawals(output),
            Self::ApproveWithdrawal { .. } => self.pack_approve_withdrawal(output),
            Self::DenyWithdrawal { .. } => self.pack_deny_withdrawal(output),
            Self::LockAssets { .. } => self.pack_accept_funds(output),
            Self::UnlockAssets { .. } => self.pack_release_funds(output),
            Self::SetAdminSigners { .. } => self.pack_set_admin_signers(output),
            Self::RemoveMultisig { .. } => self.pack_remove_multisig(output),
            Self::SetAssetsTrackingConfig { .. } => self.pack_set_assets_tracking_config(output),
            Self::UpdateAssetsWithVault { .. } => self.pack_update_assets_with_vault(output),
            Self::UpdateAssetsWithCustody { .. } => self.pack_update_assets_with_custody(output),
            Self::AddVault { .. } => self.pack_add_vault(output),
            Self::RemoveVault { .. } => self.pack_remove_vault(output),
            Self::AddCustody { .. } => self.pack_add_custody(output),
            Self::RemoveCustody { .. } => self.pack_remove_custody(output),
            Self::StartLiquidation { .. } => self.pack_start_liquidation(output),
            Self::StopLiquidation { .. } => self.pack_stop_liquidation(output),
            Self::WithdrawFees { .. } => self.pack_withdraw_fees(output),
            Self::AmmInstructionRaydium { .. } => self.pack_amm_instruction_raydium(output),
            Self::VaultInstructionRaydium { .. } => self.pack_vault_instruction_raydium(output),
            Self::AmmInstructionOrca { .. } => self.pack_amm_instruction_orca(output),
            Self::VaultInstructionOrca { .. } => self.pack_vault_instruction_orca(output),
        }
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; FundInstruction::MAX_LEN] = [0; FundInstruction::MAX_LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    pub fn unpack(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, 1)?;
        let instruction_type = FundInstructionType::try_from_primitive(input[0])
            .or(Err(ProgramError::InvalidInstructionData))?;
        match instruction_type {
            FundInstructionType::UserInit => FundInstruction::unpack_user_init(input),
            FundInstructionType::RequestDeposit => FundInstruction::unpack_request_deposit(input),
            FundInstructionType::CancelDeposit => FundInstruction::unpack_cancel_deposit(input),
            FundInstructionType::RequestWithdrawal => {
                FundInstruction::unpack_request_withdrawal(input)
            }
            FundInstructionType::CancelWithdrawal => {
                FundInstruction::unpack_cancel_withdrawal(input)
            }
            FundInstructionType::Init => FundInstruction::unpack_init(input),
            FundInstructionType::SetDepositSchedule => {
                FundInstruction::unpack_set_deposit_schedule(input)
            }
            FundInstructionType::DisableDeposits => FundInstruction::unpack_disable_deposits(input),
            FundInstructionType::ApproveDeposit => FundInstruction::unpack_approve_deposit(input),
            FundInstructionType::DenyDeposit => FundInstruction::unpack_deny_deposit(input),
            FundInstructionType::SetWithdrawalSchedule => {
                FundInstruction::unpack_set_withdrawal_schedule(input)
            }
            FundInstructionType::DisableWithdrawals => {
                FundInstruction::unpack_disable_withdrawals(input)
            }
            FundInstructionType::ApproveWithdrawal => {
                FundInstruction::unpack_approve_withdrawal(input)
            }
            FundInstructionType::DenyWithdrawal => FundInstruction::unpack_deny_withdrawal(input),
            FundInstructionType::LockAssets => FundInstruction::unpack_accept_funds(input),
            FundInstructionType::UnlockAssets => FundInstruction::unpack_release_funds(input),
            FundInstructionType::SetAdminSigners => {
                FundInstruction::unpack_set_admin_signers(input)
            }
            FundInstructionType::RemoveMultisig => FundInstruction::unpack_remove_multisig(input),
            FundInstructionType::SetAssetsTrackingConfig => {
                FundInstruction::unpack_set_assets_tracking_config(input)
            }
            FundInstructionType::UpdateAssetsWithVault => {
                FundInstruction::unpack_update_assets_with_vault(input)
            }
            FundInstructionType::UpdateAssetsWithCustody => {
                FundInstruction::unpack_update_assets_with_custody(input)
            }
            FundInstructionType::AddVault => FundInstruction::unpack_add_vault(input),
            FundInstructionType::RemoveVault => FundInstruction::unpack_remove_vault(input),
            FundInstructionType::AddCustody => FundInstruction::unpack_add_custody(input),
            FundInstructionType::RemoveCustody => FundInstruction::unpack_remove_custody(input),
            FundInstructionType::StartLiquidation => {
                FundInstruction::unpack_start_liquidation(input)
            }
            FundInstructionType::StopLiquidation => FundInstruction::unpack_stop_liquidation(input),
            FundInstructionType::WithdrawFees => FundInstruction::unpack_withdraw_fees(input),
            FundInstructionType::AmmInstructionRaydium => {
                FundInstruction::unpack_amm_instruction_raydium(input)
            }
            FundInstructionType::VaultInstructionRaydium => {
                FundInstruction::unpack_vault_instruction_raydium(input)
            }
            FundInstructionType::AmmInstructionOrca => {
                FundInstruction::unpack_amm_instruction_orca(input)
            }
            FundInstructionType::VaultInstructionOrca => {
                FundInstruction::unpack_vault_instruction_orca(input)
            }
        }
    }

    fn pack_user_init(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::USER_INIT_LEN)?;

        if let FundInstruction::UserInit = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = FundInstructionType::UserInit as u8;

            Ok(FundInstruction::USER_INIT_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_request_deposit(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::REQUEST_DEPOSIT_LEN)?;

        if let FundInstruction::RequestDeposit { amount } = self {
            let output = array_mut_ref![output, 0, FundInstruction::REQUEST_DEPOSIT_LEN];
            let (instruction_type_out, amount_out) = mut_array_refs![output, 1, 8];

            instruction_type_out[0] = FundInstructionType::RequestDeposit as u8;

            *amount_out = amount.to_le_bytes();

            Ok(FundInstruction::REQUEST_DEPOSIT_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_cancel_deposit(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::CANCEL_DEPOSIT_LEN)?;

        if let FundInstruction::CancelDeposit = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = FundInstructionType::CancelDeposit as u8;

            Ok(FundInstruction::CANCEL_DEPOSIT_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_request_withdrawal(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::REQUEST_WITHDRAWAL_LEN)?;

        if let FundInstruction::RequestWithdrawal { amount } = self {
            let output = array_mut_ref![output, 0, FundInstruction::REQUEST_WITHDRAWAL_LEN];
            let (instruction_type_out, amount_out) = mut_array_refs![output, 1, 8];

            instruction_type_out[0] = FundInstructionType::RequestWithdrawal as u8;

            *amount_out = amount.to_le_bytes();

            Ok(FundInstruction::REQUEST_WITHDRAWAL_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_cancel_withdrawal(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::CANCEL_WITHDRAWAL_LEN)?;

        if let FundInstruction::CancelWithdrawal = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = FundInstructionType::CancelWithdrawal as u8;

            Ok(FundInstruction::CANCEL_WITHDRAWAL_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_init(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::INIT_LEN)?;

        if let FundInstruction::Init { step } = self {
            let output = array_mut_ref![output, 0, FundInstruction::INIT_LEN];
            let (instruction_type_out, step_out) = mut_array_refs![output, 1, 8];

            instruction_type_out[0] = FundInstructionType::Init as u8;

            *step_out = step.to_le_bytes();

            Ok(FundInstruction::INIT_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_set_deposit_schedule(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::SET_DEPOSIT_SCHEDULE_LEN)?;

        if let FundInstruction::SetDepositSchedule { schedule } = self {
            let output = array_mut_ref![output, 0, FundInstruction::SET_DEPOSIT_SCHEDULE_LEN];
            let (
                instruction_type_out,
                start_time_out,
                end_time_out,
                approval_required_out,
                min_amount_usd_out,
                max_amount_usd_out,
                fee_out,
            ) = mut_array_refs![output, 1, 8, 8, 1, 8, 8, 8];

            instruction_type_out[0] = FundInstructionType::SetDepositSchedule as u8;

            *start_time_out = schedule.start_time.to_le_bytes();
            *end_time_out = schedule.end_time.to_le_bytes();
            pack_bool(schedule.approval_required, approval_required_out);
            *min_amount_usd_out = schedule.min_amount_usd.to_le_bytes();
            *max_amount_usd_out = schedule.max_amount_usd.to_le_bytes();
            *fee_out = schedule.fee.to_le_bytes();

            Ok(FundInstruction::SET_DEPOSIT_SCHEDULE_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_disable_deposits(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::DISABLE_DEPOSITS_LEN)?;

        if let FundInstruction::DisableDeposits = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = FundInstructionType::DisableDeposits as u8;

            Ok(FundInstruction::DISABLE_DEPOSITS_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_approve_deposit(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::APPROVE_DEPOSIT_LEN)?;

        if let FundInstruction::ApproveDeposit { amount } = self {
            let output = array_mut_ref![output, 0, FundInstruction::APPROVE_DEPOSIT_LEN];
            let (instruction_type_out, amount_out) = mut_array_refs![output, 1, 8];

            instruction_type_out[0] = FundInstructionType::ApproveDeposit as u8;

            *amount_out = amount.to_le_bytes();

            Ok(FundInstruction::APPROVE_DEPOSIT_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_deny_deposit(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::DENY_DEPOSIT_LEN)?;

        if let FundInstruction::DenyDeposit { deny_reason } = self {
            let output = array_mut_ref![output, 0, FundInstruction::DENY_DEPOSIT_LEN];
            let (instruction_type_out, deny_reason_out) = mut_array_refs![output, 1, 64];

            instruction_type_out[0] = FundInstructionType::DenyDeposit as u8;

            pack_array_string64(deny_reason, deny_reason_out);

            Ok(FundInstruction::DENY_DEPOSIT_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_set_withdrawal_schedule(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::SET_WITHDRAWAL_SCHEDULE_LEN)?;

        if let FundInstruction::SetWithdrawalSchedule { schedule } = self {
            let output = array_mut_ref![output, 0, FundInstruction::SET_WITHDRAWAL_SCHEDULE_LEN];
            let (
                instruction_type_out,
                start_time_out,
                end_time_out,
                approval_required_out,
                min_amount_usd_out,
                max_amount_usd_out,
                fee_out,
            ) = mut_array_refs![output, 1, 8, 8, 1, 8, 8, 8];

            instruction_type_out[0] = FundInstructionType::SetWithdrawalSchedule as u8;

            *start_time_out = schedule.start_time.to_le_bytes();
            *end_time_out = schedule.end_time.to_le_bytes();
            pack_bool(schedule.approval_required, approval_required_out);
            *min_amount_usd_out = schedule.min_amount_usd.to_le_bytes();
            *max_amount_usd_out = schedule.max_amount_usd.to_le_bytes();
            *fee_out = schedule.fee.to_le_bytes();

            Ok(FundInstruction::SET_WITHDRAWAL_SCHEDULE_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_disable_withdrawals(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::DISABLE_WITHDRAWALS_LEN)?;

        if let FundInstruction::DisableWithdrawals = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = FundInstructionType::DisableWithdrawals as u8;

            Ok(FundInstruction::DISABLE_WITHDRAWALS_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_approve_withdrawal(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::APPROVE_WITHDRAWAL_LEN)?;

        if let FundInstruction::ApproveWithdrawal { amount } = self {
            let output = array_mut_ref![output, 0, FundInstruction::APPROVE_WITHDRAWAL_LEN];
            let (instruction_type_out, amount_out) = mut_array_refs![output, 1, 8];

            instruction_type_out[0] = FundInstructionType::ApproveWithdrawal as u8;

            *amount_out = amount.to_le_bytes();

            Ok(FundInstruction::APPROVE_WITHDRAWAL_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_deny_withdrawal(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::DENY_WITHDRAWAL_LEN)?;

        if let FundInstruction::DenyWithdrawal { deny_reason } = self {
            let output = array_mut_ref![output, 0, FundInstruction::DENY_WITHDRAWAL_LEN];
            let (instruction_type_out, deny_reason_out) = mut_array_refs![output, 1, 64];

            instruction_type_out[0] = FundInstructionType::DenyWithdrawal as u8;

            pack_array_string64(deny_reason, deny_reason_out);

            Ok(FundInstruction::DENY_WITHDRAWAL_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_accept_funds(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::LOCK_ASSETS_LEN)?;

        if let FundInstruction::LockAssets { amount } = self {
            let output = array_mut_ref![output, 0, FundInstruction::LOCK_ASSETS_LEN];
            let (instruction_type_out, amount_out) = mut_array_refs![output, 1, 8];

            instruction_type_out[0] = FundInstructionType::LockAssets as u8;

            *amount_out = amount.to_le_bytes();

            Ok(FundInstruction::LOCK_ASSETS_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_release_funds(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::UNLOCK_ASSETS_LEN)?;

        if let FundInstruction::UnlockAssets { amount } = self {
            let output = array_mut_ref![output, 0, FundInstruction::UNLOCK_ASSETS_LEN];
            let (instruction_type_out, amount_out) = mut_array_refs![output, 1, 8];

            instruction_type_out[0] = FundInstructionType::UnlockAssets as u8;

            *amount_out = amount.to_le_bytes();

            Ok(FundInstruction::UNLOCK_ASSETS_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_set_admin_signers(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::SET_ADMIN_SIGNERS_LEN)?;

        if let FundInstruction::SetAdminSigners { min_signatures } = self {
            let output = array_mut_ref![output, 0, FundInstruction::SET_ADMIN_SIGNERS_LEN];
            let (instruction_type_out, min_signatures_out) = mut_array_refs![output, 1, 1];

            instruction_type_out[0] = FundInstructionType::SetAdminSigners as u8;
            min_signatures_out[0] = *min_signatures;

            Ok(FundInstruction::SET_ADMIN_SIGNERS_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_remove_multisig(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::REMOVE_MULTISIG_LEN)?;

        if let FundInstruction::RemoveMultisig = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = FundInstructionType::RemoveMultisig as u8;

            Ok(FundInstruction::REMOVE_MULTISIG_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_set_assets_tracking_config(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::SET_ASSETS_TRACKING_CONFIG_LEN)?;

        if let FundInstruction::SetAssetsTrackingConfig { config } = self {
            let output = array_mut_ref![output, 0, FundInstruction::SET_ASSETS_TRACKING_CONFIG_LEN];
            let (
                instruction_type_out,
                assets_limit_usd_out,
                max_update_age_sec_out,
                max_price_error_out,
                max_price_age_sec_out,
                issue_virtual_tokens_out,
            ) = mut_array_refs![output, 1, 8, 8, 8, 8, 1];

            instruction_type_out[0] = FundInstructionType::SetAssetsTrackingConfig as u8;

            *assets_limit_usd_out = config.assets_limit_usd.to_le_bytes();
            *max_update_age_sec_out = config.max_update_age_sec.to_le_bytes();
            *max_price_error_out = config.max_price_error.to_le_bytes();
            *max_price_age_sec_out = config.max_price_age_sec.to_le_bytes();
            issue_virtual_tokens_out[0] = config.issue_virtual_tokens as u8;

            Ok(FundInstruction::SET_ASSETS_TRACKING_CONFIG_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_update_assets_with_vault(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::UPDATE_ASSETS_WITH_VAULT_LEN)?;

        if let FundInstruction::UpdateAssetsWithVault = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = FundInstructionType::UpdateAssetsWithVault as u8;

            Ok(FundInstruction::UPDATE_ASSETS_WITH_VAULT_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_update_assets_with_custody(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::UPDATE_ASSETS_WITH_CUSTODY_LEN)?;

        if let FundInstruction::UpdateAssetsWithCustody = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = FundInstructionType::UpdateAssetsWithCustody as u8;

            Ok(FundInstruction::UPDATE_ASSETS_WITH_CUSTODY_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_add_vault(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::ADD_VAULT_LEN)?;

        if let FundInstruction::AddVault {
            target_hash,
            vault_id,
            vault_type,
        } = self
        {
            let output = array_mut_ref![output, 0, FundInstruction::ADD_VAULT_LEN];
            let (instruction_type_out, target_hash_out, vault_id_out, vault_type_out) =
                mut_array_refs![output, 1, 8, 4, 1];

            instruction_type_out[0] = FundInstructionType::AddVault as u8;

            *target_hash_out = target_hash.to_le_bytes();
            *vault_id_out = vault_id.to_le_bytes();
            vault_type_out[0] = *vault_type as u8;

            Ok(FundInstruction::ADD_VAULT_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_remove_vault(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::REMOVE_VAULT_LEN)?;

        if let FundInstruction::RemoveVault {
            target_hash,
            vault_type,
        } = self
        {
            let output = array_mut_ref![output, 0, FundInstruction::REMOVE_VAULT_LEN];
            let (instruction_type_out, target_hash_out, vault_type_out) =
                mut_array_refs![output, 1, 8, 1];

            instruction_type_out[0] = FundInstructionType::RemoveVault as u8;

            *target_hash_out = target_hash.to_le_bytes();
            vault_type_out[0] = *vault_type as u8;

            Ok(FundInstruction::REMOVE_VAULT_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_add_custody(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::ADD_CUSTODY_LEN)?;

        if let FundInstruction::AddCustody {
            target_hash,
            custody_id,
            custody_type,
        } = self
        {
            let output = array_mut_ref![output, 0, FundInstruction::ADD_CUSTODY_LEN];
            let (instruction_type_out, target_hash_out, custody_id_out, custody_type_out) =
                mut_array_refs![output, 1, 8, 4, 1];

            instruction_type_out[0] = FundInstructionType::AddCustody as u8;

            *target_hash_out = target_hash.to_le_bytes();
            *custody_id_out = custody_id.to_le_bytes();
            custody_type_out[0] = *custody_type as u8;

            Ok(FundInstruction::ADD_CUSTODY_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_remove_custody(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::REMOVE_CUSTODY_LEN)?;

        if let FundInstruction::RemoveCustody {
            target_hash,
            custody_type,
        } = self
        {
            let output = array_mut_ref![output, 0, FundInstruction::REMOVE_CUSTODY_LEN];
            let (instruction_type_out, target_hash_out, custody_type_out) =
                mut_array_refs![output, 1, 8, 1];

            instruction_type_out[0] = FundInstructionType::RemoveCustody as u8;

            *target_hash_out = target_hash.to_le_bytes();
            custody_type_out[0] = *custody_type as u8;

            Ok(FundInstruction::REMOVE_CUSTODY_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_start_liquidation(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::START_LIQUIDATION_LEN)?;

        if let FundInstruction::StartLiquidation = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = FundInstructionType::StartLiquidation as u8;

            Ok(FundInstruction::START_LIQUIDATION_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_stop_liquidation(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::STOP_LIQUIDATION_LEN)?;

        if let FundInstruction::StopLiquidation = self {
            let instruction_type_out = array_mut_ref![output, 0, 1];

            instruction_type_out[0] = FundInstructionType::StopLiquidation as u8;

            Ok(FundInstruction::STOP_LIQUIDATION_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_withdraw_fees(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, FundInstruction::WITHDRAW_FEES_LEN)?;

        if let FundInstruction::WithdrawFees { amount } = self {
            let output = array_mut_ref![output, 0, FundInstruction::WITHDRAW_FEES_LEN];
            let (instruction_type_out, amount_out) = mut_array_refs![output, 1, 8];

            instruction_type_out[0] = FundInstructionType::WithdrawFees as u8;

            *amount_out = amount.to_le_bytes();

            Ok(FundInstruction::WITHDRAW_FEES_LEN)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_amm_instruction_raydium(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        if let FundInstruction::AmmInstructionRaydium { instruction } = self {
            check_data_len(output, 1)?;

            let instruction_type_out = array_mut_ref![output, 0, 1];
            instruction_type_out[0] = FundInstructionType::AmmInstructionRaydium as u8;

            Ok(instruction.pack(&mut output[1..])? + 1)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_vault_instruction_raydium(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        if let FundInstruction::VaultInstructionRaydium { instruction } = self {
            check_data_len(output, 1)?;

            let instruction_type_out = array_mut_ref![output, 0, 1];
            instruction_type_out[0] = FundInstructionType::VaultInstructionRaydium as u8;

            Ok(instruction.pack(&mut output[1..])? + 1)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_amm_instruction_orca(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        if let FundInstruction::AmmInstructionOrca { instruction } = self {
            check_data_len(output, 1)?;

            let instruction_type_out = array_mut_ref![output, 0, 1];
            instruction_type_out[0] = FundInstructionType::AmmInstructionOrca as u8;

            Ok(instruction.pack(&mut output[1..])? + 1)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn pack_vault_instruction_orca(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        if let FundInstruction::VaultInstructionOrca { instruction } = self {
            check_data_len(output, 1)?;

            let instruction_type_out = array_mut_ref![output, 0, 1];
            instruction_type_out[0] = FundInstructionType::VaultInstructionOrca as u8;

            Ok(instruction.pack(&mut output[1..])? + 1)
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    fn unpack_user_init(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::USER_INIT_LEN)?;
        Ok(Self::UserInit)
    }

    fn unpack_request_deposit(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::REQUEST_DEPOSIT_LEN)?;
        Ok(Self::RequestDeposit {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_cancel_deposit(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::CANCEL_DEPOSIT_LEN)?;
        Ok(Self::CancelDeposit)
    }

    fn unpack_request_withdrawal(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::REQUEST_WITHDRAWAL_LEN)?;
        Ok(Self::RequestWithdrawal {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_cancel_withdrawal(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::CANCEL_WITHDRAWAL_LEN)?;
        Ok(Self::CancelWithdrawal)
    }

    fn unpack_init(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::INIT_LEN)?;
        Ok(Self::Init {
            step: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_set_deposit_schedule(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::SET_DEPOSIT_SCHEDULE_LEN)?;

        let input = array_ref![input, 1, FundInstruction::SET_DEPOSIT_SCHEDULE_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (start_time, end_time, approval_required, min_amount_usd, max_amount_usd, fee) =
            array_refs![input, 8, 8, 1, 8, 8, 8];

        Ok(Self::SetDepositSchedule {
            schedule: FundSchedule {
                start_time: i64::from_le_bytes(*start_time),
                end_time: i64::from_le_bytes(*end_time),
                approval_required: unpack_bool(approval_required)?,
                min_amount_usd: f64::from_le_bytes(*min_amount_usd),
                max_amount_usd: f64::from_le_bytes(*max_amount_usd),
                fee: f64::from_le_bytes(*fee),
            },
        })
    }

    fn unpack_disable_deposits(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::DISABLE_DEPOSITS_LEN)?;
        Ok(Self::DisableDeposits)
    }

    fn unpack_approve_deposit(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::APPROVE_DEPOSIT_LEN)?;
        Ok(Self::ApproveDeposit {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_deny_deposit(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::DENY_DEPOSIT_LEN)?;
        Ok(Self::DenyDeposit {
            deny_reason: unpack_array_string64(array_ref![input, 1, 64])?,
        })
    }

    fn unpack_set_withdrawal_schedule(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::SET_WITHDRAWAL_SCHEDULE_LEN)?;

        let input = array_ref![input, 1, FundInstruction::SET_WITHDRAWAL_SCHEDULE_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (start_time, end_time, approval_required, min_amount_usd, max_amount_usd, fee) =
            array_refs![input, 8, 8, 1, 8, 8, 8];

        Ok(Self::SetWithdrawalSchedule {
            schedule: FundSchedule {
                start_time: i64::from_le_bytes(*start_time),
                end_time: i64::from_le_bytes(*end_time),
                approval_required: unpack_bool(approval_required)?,
                min_amount_usd: f64::from_le_bytes(*min_amount_usd),
                max_amount_usd: f64::from_le_bytes(*max_amount_usd),
                fee: f64::from_le_bytes(*fee),
            },
        })
    }

    fn unpack_disable_withdrawals(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::DISABLE_WITHDRAWALS_LEN)?;
        Ok(Self::DisableWithdrawals)
    }

    fn unpack_approve_withdrawal(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::APPROVE_WITHDRAWAL_LEN)?;
        Ok(Self::ApproveWithdrawal {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_deny_withdrawal(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::DENY_WITHDRAWAL_LEN)?;
        Ok(Self::DenyWithdrawal {
            deny_reason: unpack_array_string64(array_ref![input, 1, 64])?,
        })
    }

    fn unpack_accept_funds(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::LOCK_ASSETS_LEN)?;
        Ok(Self::LockAssets {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_release_funds(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::UNLOCK_ASSETS_LEN)?;
        Ok(Self::UnlockAssets {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_set_admin_signers(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::SET_ADMIN_SIGNERS_LEN)?;

        let input = array_ref![input, 1, FundInstruction::SET_ADMIN_SIGNERS_LEN - 1];

        Ok(Self::SetAdminSigners {
            min_signatures: input[0],
        })
    }

    fn unpack_remove_multisig(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::REMOVE_MULTISIG_LEN)?;
        Ok(Self::RemoveMultisig)
    }

    fn unpack_set_assets_tracking_config(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::SET_ASSETS_TRACKING_CONFIG_LEN)?;

        let input = array_ref![
            input,
            1,
            FundInstruction::SET_ASSETS_TRACKING_CONFIG_LEN - 1
        ];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            assets_limit_usd,
            max_update_age_sec,
            max_price_error,
            max_price_age_sec,
            issue_virtual_tokens,
        ) = array_refs![input, 8, 8, 8, 8, 1];

        Ok(Self::SetAssetsTrackingConfig {
            config: FundAssetsTrackingConfig {
                assets_limit_usd: f64::from_le_bytes(*assets_limit_usd),
                max_update_age_sec: u64::from_le_bytes(*max_update_age_sec),
                max_price_error: f64::from_le_bytes(*max_price_error),
                max_price_age_sec: u64::from_le_bytes(*max_price_age_sec),
                issue_virtual_tokens: unpack_bool(issue_virtual_tokens)?,
            },
        })
    }

    fn unpack_update_assets_with_vault(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::UPDATE_ASSETS_WITH_VAULT_LEN)?;
        Ok(Self::UpdateAssetsWithVault)
    }

    fn unpack_update_assets_with_custody(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::UPDATE_ASSETS_WITH_CUSTODY_LEN)?;
        Ok(Self::UpdateAssetsWithCustody)
    }

    fn unpack_add_vault(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::ADD_VAULT_LEN)?;

        let input = array_ref![input, 1, FundInstruction::ADD_VAULT_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (target_hash, vault_id, vault_type) = array_refs![input, 8, 4, 1];

        Ok(Self::AddVault {
            target_hash: u64::from_le_bytes(*target_hash),
            vault_id: u32::from_le_bytes(*vault_id),
            vault_type: FundVaultType::try_from_primitive(vault_type[0])
                .or(Err(ProgramError::InvalidInstructionData))?,
        })
    }

    fn unpack_remove_vault(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::REMOVE_VAULT_LEN)?;

        let input = array_ref![input, 1, FundInstruction::REMOVE_VAULT_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (target_hash, vault_type) = array_refs![input, 8, 1];

        Ok(Self::RemoveVault {
            target_hash: u64::from_le_bytes(*target_hash),
            vault_type: FundVaultType::try_from_primitive(vault_type[0])
                .or(Err(ProgramError::InvalidInstructionData))?,
        })
    }

    fn unpack_add_custody(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::ADD_CUSTODY_LEN)?;

        let input = array_ref![input, 1, FundInstruction::ADD_CUSTODY_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (target_hash, custody_id, custody_type) = array_refs![input, 8, 4, 1];

        Ok(Self::AddCustody {
            target_hash: u64::from_le_bytes(*target_hash),
            custody_id: u32::from_le_bytes(*custody_id),
            custody_type: FundCustodyType::try_from_primitive(custody_type[0])
                .or(Err(ProgramError::InvalidInstructionData))?,
        })
    }

    fn unpack_remove_custody(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::REMOVE_CUSTODY_LEN)?;

        let input = array_ref![input, 1, FundInstruction::REMOVE_CUSTODY_LEN - 1];
        #[allow(clippy::ptr_offset_with_cast)]
        let (target_hash, custody_type) = array_refs![input, 8, 1];

        Ok(Self::RemoveCustody {
            target_hash: u64::from_le_bytes(*target_hash),
            custody_type: FundCustodyType::try_from_primitive(custody_type[0])
                .or(Err(ProgramError::InvalidInstructionData))?,
        })
    }

    fn unpack_start_liquidation(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::START_LIQUIDATION_LEN)?;
        Ok(Self::StartLiquidation)
    }

    fn unpack_stop_liquidation(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::STOP_LIQUIDATION_LEN)?;
        Ok(Self::StopLiquidation)
    }

    fn unpack_withdraw_fees(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        check_data_len(input, FundInstruction::WITHDRAW_FEES_LEN)?;
        Ok(Self::WithdrawFees {
            amount: u64::from_le_bytes(*array_ref![input, 1, 8]),
        })
    }

    fn unpack_amm_instruction_raydium(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        Ok(Self::AmmInstructionRaydium {
            instruction: AmmInstruction::unpack(&input[1..])?,
        })
    }

    fn unpack_vault_instruction_raydium(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        Ok(Self::VaultInstructionRaydium {
            instruction: VaultInstruction::unpack(&input[1..])?,
        })
    }

    fn unpack_amm_instruction_orca(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        Ok(Self::AmmInstructionOrca {
            instruction: AmmInstruction::unpack(&input[1..])?,
        })
    }

    fn unpack_vault_instruction_orca(input: &[u8]) -> Result<FundInstruction, ProgramError> {
        Ok(Self::VaultInstructionOrca {
            instruction: VaultInstruction::unpack(&input[1..])?,
        })
    }
}

impl std::fmt::Display for FundInstructionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            FundInstructionType::UserInit => write!(f, "UserInit"),
            FundInstructionType::RequestDeposit => write!(f, "RequestDeposit"),
            FundInstructionType::CancelDeposit => write!(f, "CancelDeposit"),
            FundInstructionType::RequestWithdrawal => write!(f, "RequestWithdrawal"),
            FundInstructionType::CancelWithdrawal => write!(f, "CancelWithdrawal"),
            FundInstructionType::Init => write!(f, "Init"),
            FundInstructionType::SetDepositSchedule => write!(f, "SetDepositSchedule"),
            FundInstructionType::DisableDeposits => write!(f, "DisableDeposits"),
            FundInstructionType::ApproveDeposit => write!(f, "ApproveDeposit"),
            FundInstructionType::DenyDeposit => write!(f, "DenyDeposit"),
            FundInstructionType::SetWithdrawalSchedule => write!(f, "SetWithdrawalSchedule"),
            FundInstructionType::DisableWithdrawals => write!(f, "DisableWithdrawals"),
            FundInstructionType::ApproveWithdrawal => write!(f, "ApproveWithdrawal"),
            FundInstructionType::DenyWithdrawal => write!(f, "DenyWithdrawal"),
            FundInstructionType::LockAssets => write!(f, "LockAssets"),
            FundInstructionType::UnlockAssets => write!(f, "UnlockAssets"),
            FundInstructionType::SetAdminSigners => write!(f, "SetAdminSigners"),
            FundInstructionType::RemoveMultisig => write!(f, "RemoveMultisig"),
            FundInstructionType::SetAssetsTrackingConfig => write!(f, "SetAssetsTrackingConfig"),
            FundInstructionType::UpdateAssetsWithVault => write!(f, "UpdateAssetsWithVault"),
            FundInstructionType::UpdateAssetsWithCustody => write!(f, "UpdateAssetsWithCustody"),
            FundInstructionType::AddVault => write!(f, "AddVault"),
            FundInstructionType::RemoveVault => write!(f, "RemoveVault"),
            FundInstructionType::AddCustody => write!(f, "AddCustody"),
            FundInstructionType::RemoveCustody => write!(f, "RemoveCustody"),
            FundInstructionType::StartLiquidation => write!(f, "StartLiquidation"),
            FundInstructionType::StopLiquidation => write!(f, "StopLiquidation"),
            FundInstructionType::WithdrawFees => write!(f, "WithdrawFees"),
            FundInstructionType::AmmInstructionRaydium => write!(f, "AmmInstructionRaydium"),
            FundInstructionType::VaultInstructionRaydium => write!(f, "VaultInstructionRaydium"),
            FundInstructionType::AmmInstructionOrca => write!(f, "AmmInstructionOrca"),
            FundInstructionType::VaultInstructionOrca => write!(f, "VaultInstructionOrca"),
        }
    }
}
