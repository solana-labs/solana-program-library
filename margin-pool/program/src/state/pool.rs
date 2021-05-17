use crate::error::MarginPoolError;
use crate::state::fees::Fees;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
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
    const LEN: usize = 450;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 450];
        let (
            version,
            nonce,
            token_lp,
            token_a,
            token_b,
            pool_mint,
            token_a_mint,
            token_b_mint,
            token_lp_mint,
            token_swap,
            escrow_a,
            escrow_b,
            fees,
            token_program_id,
            token_swap_program_id,
        ) = mut_array_refs![
            output,
            1,
            1,
            32,
            32,
            32,
            32,
            32,
            32,
            32,
            32,
            32,
            32,
            Fees::LEN,
            32,
            32
        ];

        *version = self.version.to_le_bytes();
        *nonce = self.nonce.to_le_bytes();
        token_lp.copy_from_slice(self.token_lp.as_ref());
        token_a.copy_from_slice(self.token_a.as_ref());
        token_b.copy_from_slice(self.token_b.as_ref());
        pool_mint.copy_from_slice(self.pool_mint.as_ref());
        token_a_mint.copy_from_slice(self.token_a_mint.as_ref());
        token_b_mint.copy_from_slice(self.token_b_mint.as_ref());
        token_lp_mint.copy_from_slice(self.token_lp_mint.as_ref());
        token_swap.copy_from_slice(self.token_swap.as_ref());
        escrow_a.copy_from_slice(self.escrow_a.as_ref());
        escrow_a.copy_from_slice(self.escrow_a.as_ref());
        escrow_b.copy_from_slice(self.escrow_b.as_ref());
        self.fees.pack_into_slice(fees);
        token_program_id.copy_from_slice(self.token_program_id.as_ref());
        token_swap_program_id.copy_from_slice(self.token_swap_program_id.as_ref());
    }

    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, 450];

        let (
            version,
            nonce,
            token_lp,
            token_a,
            token_b,
            pool_mint,
            token_a_mint,
            token_b_mint,
            token_lp_mint,
            token_swap,
            escrow_a,
            escrow_b,
            fees,
            token_program_id,
            token_swap_program_id,
        ) = array_refs![
            input,
            1,
            1,
            32,
            32,
            32,
            32,
            32,
            32,
            32,
            32,
            32,
            32,
            Fees::LEN,
            32,
            32
        ];

        Ok(Self {
            version: u8::from_le_bytes(*version),
            nonce: u8::from_le_bytes(*nonce),
            token_lp: Pubkey::new_from_array(*token_lp),
            token_a: Pubkey::new_from_array(*token_a),
            token_b: Pubkey::new_from_array(*token_b),
            pool_mint: Pubkey::new_from_array(*pool_mint),
            token_a_mint: Pubkey::new_from_array(*token_a_mint),
            token_b_mint: Pubkey::new_from_array(*token_b_mint),
            token_lp_mint: Pubkey::new_from_array(*token_lp_mint),
            token_swap: Pubkey::new_from_array(*token_swap),
            escrow_a: Pubkey::new_from_array(*escrow_a),
            escrow_b: Pubkey::new_from_array(*escrow_b),
            fees: Fees::unpack_from_slice(fees).unwrap(),
            token_program_id: Pubkey::new_from_array(*token_program_id),
            token_swap_program_id: Pubkey::new_from_array(*token_swap_program_id),
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pack_margin_pool() {
        let pool = MarginPool {
            version: 1,
            nonce: 2,
            token_lp: Pubkey::new_from_array([1u8; 32]),
            token_a: Pubkey::new_from_array([2u8; 32]),
            token_b: Pubkey::new_from_array([3u8; 32]),
            pool_mint: Pubkey::new_from_array([4u8; 32]),
            token_a_mint: Pubkey::new_from_array([5u8; 32]),
            token_b_mint: Pubkey::new_from_array([6u8; 32]),
            token_lp_mint: Pubkey::new_from_array([7u8; 32]),
            token_swap: Pubkey::new_from_array([8u8; 32]),
            escrow_a: Pubkey::new_from_array([9u8; 32]),
            escrow_b: Pubkey::new_from_array([10u8; 32]),
            fees: Fees {
                position_fee_numerator: 1,
                position_fee_denominator: 11,
                owner_withdraw_fee_numerator: 2,
                owner_withdraw_fee_denominator: 12,
                owner_position_fee_numerator: 3,
                owner_position_fee_denominator: 13,
                host_position_fee_numerator: 4,
                host_position_fee_denominator: 14,
            },
            token_program_id: Pubkey::new_from_array([11u8; 32]),
            token_swap_program_id: Pubkey::new_from_array([12u8; 32]),
        };

        let mut packed = [0u8; MarginPool::LEN];
        pool.pack_into_slice(&mut packed);

        let unpacked = MarginPool::unpack_from_slice(&packed).unwrap();

        assert_eq!(pool, unpacked);
    }
}
