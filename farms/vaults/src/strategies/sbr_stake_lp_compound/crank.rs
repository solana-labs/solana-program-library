//! Vault Crank instruction handler

use {
    crate::{
        strategies::sbr_stake_lp_compound::{
            crank1::crank1, crank2::crank2, crank3::crank3, crank4::crank4, crank5::crank5,
        },
        traits::Crank,
    },
    solana_farm_sdk::{instruction::vault::VaultInstruction, vault::Vault},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

impl Crank for VaultInstruction {
    fn crank(vault: &Vault, accounts: &[AccountInfo], step: u64) -> ProgramResult {
        match step {
            1 => crank1(vault, accounts),
            2 => crank2(vault, accounts),
            3 => crank3(vault, accounts),
            4 => crank4(vault, accounts),
            5 => crank5(vault, accounts),
            _ => {
                msg!("Error: Invalid Crank step");
                Err(ProgramError::InvalidArgument)
            }
        }
    }
}
