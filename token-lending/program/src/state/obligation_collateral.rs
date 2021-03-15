use super::*;
use crate::{
    error::LendingError,
    math::{Decimal, Rate, TryAdd, TryDiv, TryMul, TrySub},
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    clock::Slot,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use std::convert::TryInto;

/// Obligation collateral state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObligationCollateral {
    /// Version of the obligation collateral
    pub version: u8,
    /// Last slot when market value updated; set to 0 if deposited tokens changed
    pub last_update_slot: Slot,
    /// Obligation the collateral is associated with
    pub obligation: Pubkey,
    /// Reserve which collateral tokens were deposited into
    pub deposit_reserve: Pubkey,
    /// Mint address of the tokens for this obligation collateral
    pub token_mint: Pubkey,
    /// Amount of collateral tokens deposited for an obligation
    pub deposited_tokens: u64,
    /// Market value of collateral
    pub market_value: Decimal,
}

/// Create new obligation collateral
pub struct NewObligationCollateralParams {
    /// Obligation address
    pub obligation: Pubkey,
    /// Deposit reserve address
    pub deposit_reserve: Pubkey,
    /// Obligation token mint address
    pub token_mint: Pubkey,
}

impl ObligationCollateral {
    /// Create new obligation collateral
    pub fn new(params: NewObligationCollateralParams) -> Self {
        let NewObligationCollateralParams {
            obligation,
            deposit_reserve,
            token_mint,
        } = params;

        Self {
            version: PROGRAM_VERSION,
            last_update_slot: 0,
            obligation,
            deposit_reserve,
            token_mint,
            deposited_tokens: 0,
            market_value: Decimal::zero(),
        }
    }

    pub fn deposit(&mut self, collateral_amount: u64) -> Result<u64, ProgramError> {
        self.deposited_tokens = self
            .deposited_tokens
            .checked_add(collateral_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(self.deposited_tokens)
    }

    pub fn withdraw(&mut self, collateral_amount: u64) -> Result<u64, ProgramError> {
        self.deposited_tokens = self
            .deposited_tokens
            .checked_sub(collateral_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(self.deposited_tokens)
    }

    pub fn update_market_value(
        &mut self,
        collateral_exchange_rate: CollateralExchangeRate,
        converter: impl TokenConverter,
        liquidity_token_mint: &Pubkey,
    ) -> ProgramResult {
        let liquidity_amount = collateral_exchange_rate
            .decimal_collateral_to_liquidity(self.deposited_tokens.into())?;
        self.market_value = converter.convert(liquidity_amount, liquidity_token_mint)?;
        Ok(())
    }

    /// Amount of obligation tokens for given collateral
    pub fn collateral_to_obligation_token_amount(
        &self,
        collateral_amount: u64,
        obligation_token_supply: u64,
    ) -> Result<u64, ProgramError> {
        let withdraw_pct = Decimal::from(collateral_amount).try_div(self.deposited_tokens)?;
        withdraw_pct
            .try_mul(obligation_token_supply)?
            .try_floor_u64()
    }

    /// Return slots elapsed
    pub fn slots_elapsed(&self, slot: Slot) -> Result<u64, ProgramError> {
        let slots_elapsed = slot
            .checked_sub(self.last_update_slot)
            .ok_or(LendingError::MathOverflow)?;
        Ok(slots_elapsed)
    }

    /// Set last update slot
    pub fn update_slot(&mut self, slot: Slot) {
        self.last_update_slot = slot;
    }

    /// Set last update slot to 0
    pub fn mark_stale(&mut self) {
        self.update_slot(0);
    }

    /// Check if last update slot is recent
    pub fn is_stale(&self, slot: Slot) -> Result<bool, ProgramError> {
        Ok(self.last_update_slot == 0 || self.slots_elapsed(slot)? > STALE_AFTER_SLOTS)
    }
}

impl Sealed for ObligationCollateral {}
impl IsInitialized for ObligationCollateral {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

const OBLIGATION_COLLATERAL_LEN: usize = 257; // 1 + 32 + 32 + 32 + 8 + 8 + 16 + 128
impl Pack for ObligationCollateral {
    const LEN: usize = OBLIGATION_COLLATERAL_LEN;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, OBLIGATION_COLLATERAL_LEN];
        let (
            version,
            last_update_slot,
            obligation,
            deposit_reserve,
            token_mint,
            deposited_tokens,
            market_value,
            _padding,
        ) = mut_array_refs![output, 1, 8, PUBKEY_LEN, PUBKEY_LEN, PUBKEY_LEN, 8, 16, 128];

        *version = self.version.to_le_bytes();
        *last_update_slot = self.last_update_slot.to_le_bytes();
        obligation.copy_from_slice(self.obligation.as_ref());
        deposit_reserve.copy_from_slice(self.deposit_reserve.as_ref());
        token_mint.copy_from_slice(self.token_mint.as_ref());
        *deposited_tokens = self.deposited_tokens.to_le_bytes();
        pack_decimal(self.market_value, market_value);
    }

    /// Unpacks a byte buffer into a [ObligationInfo](struct.ObligationInfo.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, OBLIGATION_COLLATERAL_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            last_update_slot,
            obligation,
            deposit_reserve,
            token_mint,
            deposited_tokens,
            market_value,
            _padding,
        ) = array_refs![input, 1, 8, PUBKEY_LEN, PUBKEY_LEN, PUBKEY_LEN, 8, 16, 128];

        Ok(Self {
            version: u8::from_le_bytes(*version),
            last_update_slot: u64::from_le_bytes(*last_update_slot),
            obligation: Pubkey::new_from_array(*obligation),
            deposit_reserve: Pubkey::new_from_array(*deposit_reserve),
            token_mint: Pubkey::new_from_array(*token_mint),
            deposited_tokens: u64::from_le_bytes(*deposited_tokens),
            market_value: unpack_decimal(market_value),
        })
    }
}
