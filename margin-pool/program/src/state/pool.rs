use crate::error::MarginPoolError;
use crate::state::fees::Fees;
use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{IsInitialized, Pack, Sealed};
use solana_program::pubkey::Pubkey;
use spl_token::state::{Account as TokenAccount, Mint as TokenMint};

use super::UNINITIALIZED_VERSION;

/// Margin Pool
#[repr(C)]
#[derive(Debug, Default, PartialEq)]
pub struct MarginPool {
    /// version of the margin pool
    pub version: u8,

    /// Nonce used in program address.
    /// The program address is created deterministically with the nonce,
    /// swap program id, and swap account pubkey.  This program address has
    /// authority over the swap's token A account, token B account, and pool
    /// token mint.
    pub nonce: u8,

    /// Token LP pool account
    pub token_lp: Pubkey,
    /// Token A - first component of the swap basket
    pub token_a: Pubkey,
    /// Token B - second component of the swap basket
    pub token_b: Pubkey,

    /// Pool tokens are issued when LP tokens are deposited.
    pub pool_mint: Pubkey,

    /// Mint information for token A
    pub token_a_mint: Pubkey,
    /// Mint information for token B
    pub token_b_mint: Pubkey,
    /// Mint information for token LP
    pub token_lp_mint: Pubkey,
    /// token swap pool
    pub token_swap: Pubkey,
    /// Escrow account for A
    pub escrow_a: Pubkey,
    /// Escrow account for B
    pub escrow_b: Pubkey,
    /// Pool fees
    pub fees: Fees,
    /// Program ID of the tokens being exchanged.
    pub token_program_id: Pubkey,
    /// Program ID of the token swap pool.
    pub token_swap_program_id: Pubkey,
}

impl Pack for MarginPool {
    const LEN: usize = 291;
    fn unpack_from_slice(_input: &[u8]) -> Result<Self, ProgramError> {
        unimplemented!();
    }
    fn pack_into_slice(&self, _output: &mut [u8]) {
        unimplemented!();
    }
}

impl Sealed for MarginPool {}
impl IsInitialized for MarginPool {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

impl MarginPool {
    fn get_lp_balance(&self, token_lp_info: &AccountInfo) -> Result<u64, ProgramError> {
        if *token_lp_info.key != self.token_lp {
            return Err(ProgramError::InvalidArgument);
        }
        let token_data = TokenAccount::unpack_from_slice(&token_lp_info.data.borrow())?;
        Ok(token_data.amount)
    }
    fn get_pool_balance(&self, pool_mint_info: &AccountInfo) -> Result<u64, ProgramError> {
        if *pool_mint_info.key != self.pool_mint {
            return Err(ProgramError::InvalidArgument);
        }
        let mint_data = TokenMint::unpack_from_slice(&pool_mint_info.data.borrow())?;
        Ok(mint_data.supply)
    }

    pub fn lp_token_to_pool_token(
        &self,
        token_lp_info: &AccountInfo,
        pool_mint_info: &AccountInfo,
        amount: u64,
    ) -> Result<u64, ProgramError> {
        let lp_balance = self.get_lp_balance(token_lp_info)?;
        let pool_balance = self.get_pool_balance(pool_mint_info)?;
        Ok(amount
            .checked_mul(pool_balance)
            .ok_or(MarginPoolError::CalculationFailure)?
            .checked_div(lp_balance)
            .ok_or(MarginPoolError::CalculationFailure)?)
    }

    pub fn pool_token_to_lp_token(
        &self,
        token_lp_info: &AccountInfo,
        pool_mint_info: &AccountInfo,
        amount: u64,
    ) -> Result<u64, ProgramError> {
        let lp_balance = self.get_lp_balance(token_lp_info)?;
        let pool_balance = self.get_pool_balance(pool_mint_info)?;
        Ok(amount
            .checked_mul(lp_balance)
            .ok_or(MarginPoolError::CalculationFailure)?
            .checked_div(pool_balance)
            .ok_or(MarginPoolError::CalculationFailure)?)
    }
}
