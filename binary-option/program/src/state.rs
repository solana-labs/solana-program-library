use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::error::BettingPoolError;
use borsh::{BorshDeserialize, BorshSerialize};

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Debug, Clone)]
pub struct BettingPool {
    pub decimals: u8,
    pub circulation: u64,
    pub settled: bool,
    pub escrow_mint_account_pubkey: Pubkey,
    pub escrow_account_pubkey: Pubkey,
    pub long_mint_account_pubkey: Pubkey,
    pub short_mint_account_pubkey: Pubkey,
    pub owner: Pubkey,
    pub winning_side_pubkey: Pubkey,
}

impl BettingPool {
    pub const LEN: usize = 202;

    pub fn from_account_info(a: &AccountInfo) -> Result<BettingPool, ProgramError> {
        let betting_pool = BettingPool::try_from_slice(&a.data.borrow_mut())?;
        Ok(betting_pool)
    }

    pub fn increment_supply(&mut self, n: u64) {
        self.circulation += n;
    }

    pub fn decrement_supply(&mut self, n: u64) -> ProgramResult {
        if self.circulation < n {
            return Err(BettingPoolError::InvalidSupply.into());
        }
        self.circulation -= n;
        Ok(())
    }
}
