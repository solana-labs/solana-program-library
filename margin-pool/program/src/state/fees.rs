use crate::error::MarginPoolError;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{Pack, Sealed},
};

/// Pool fees
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Fees {
    /// Per-slot position fee numerator
    pub position_fee_numerator: u64,
    /// Per-slot position fee denominator
    pub position_fee_denominator: u64,
    /// Fee charged on LP on funds withdrawal numerator
    pub owner_withdraw_fee_numerator: u64,
    /// Fee charged on LP on funds withdrawal denominator
    pub owner_withdraw_fee_denominator: u64,
    /// Part of a position fee transferred to the owner, numerator
    pub owner_position_fee_numerator: u64,
    /// Part of a position fee transferred to the owner, denominator
    pub owner_position_fee_denominator: u64,
    /// Part of a position fee transferred to the position opening host, numerator
    pub host_position_fee_numerator: u64,
    /// Part of a position fee transferred to the position opening host, denominator
    pub host_position_fee_denominator: u64,
}

impl Fees {
    pub fn withdrawal(&self, amount: u64) -> Result<u64, ProgramError> {
        Ok(amount
            .checked_mul(self.owner_withdraw_fee_numerator)
            .ok_or(MarginPoolError::CalculationFailure)?
            .checked_div(self.owner_withdraw_fee_denominator)
            .ok_or(MarginPoolError::CalculationFailure)?)
    }
}

impl Sealed for Fees {}
impl Pack for Fees {
    const LEN: usize = 64;
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 64];
        let (
            position_fee_numerator,
            position_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            owner_position_fee_numerator,
            owner_position_fee_denominator,
            host_position_fee_numerator,
            host_position_fee_denominator,
        ) = mut_array_refs![output, 8, 8, 8, 8, 8, 8, 8, 8];
        *position_fee_numerator = self.position_fee_numerator.to_le_bytes();
        *position_fee_denominator = self.position_fee_denominator.to_le_bytes();
        *owner_withdraw_fee_numerator = self.owner_withdraw_fee_numerator.to_le_bytes();
        *owner_withdraw_fee_denominator = self.owner_withdraw_fee_denominator.to_le_bytes();
        *owner_position_fee_numerator = self.owner_position_fee_numerator.to_le_bytes();
        *owner_position_fee_denominator = self.owner_position_fee_denominator.to_le_bytes();
        *host_position_fee_numerator = self.host_position_fee_numerator.to_le_bytes();
        *host_position_fee_denominator = self.host_position_fee_denominator.to_le_bytes();
    }

    fn unpack_from_slice(input: &[u8]) -> Result<Fees, ProgramError> {
        let input = array_ref![input, 0, 64];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            position_fee_numerator,
            position_fee_denominator,
            owner_withdraw_fee_numerator,
            owner_withdraw_fee_denominator,
            owner_position_fee_numerator,
            owner_position_fee_denominator,
            host_position_fee_numerator,
            host_position_fee_denominator,
        ) = array_refs![input, 8, 8, 8, 8, 8, 8, 8, 8];
        Ok(Self {
            position_fee_numerator: u64::from_le_bytes(*position_fee_numerator),
            position_fee_denominator: u64::from_le_bytes(*position_fee_denominator),
            owner_withdraw_fee_numerator: u64::from_le_bytes(*owner_withdraw_fee_numerator),
            owner_withdraw_fee_denominator: u64::from_le_bytes(*owner_withdraw_fee_denominator),
            owner_position_fee_numerator: u64::from_le_bytes(*owner_position_fee_numerator),
            owner_position_fee_denominator: u64::from_le_bytes(*owner_position_fee_denominator),
            host_position_fee_numerator: u64::from_le_bytes(*host_position_fee_numerator),
            host_position_fee_denominator: u64::from_le_bytes(*host_position_fee_denominator),
        })
    }
}
