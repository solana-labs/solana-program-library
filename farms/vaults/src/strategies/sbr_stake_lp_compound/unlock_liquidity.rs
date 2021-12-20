//! Unlock Liquidity in the Vault instruction handler

use {
    crate::traits::UnlockLiquidity,
    solana_farm_sdk::{instruction::vault::VaultInstruction, vault::Vault},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

impl UnlockLiquidity for VaultInstruction {
    fn unlock_liquidity(_vault: &Vault, _accounts: &[AccountInfo], _amount: u64) -> ProgramResult {
        msg!("Error: Liquidity Unlock is not required for this Vault");
        Err(ProgramError::InvalidArgument)
    }
}
