use crate::{
    error::LendingError,
    math::{Decimal, WAD},
};
use arrayref::{array_refs, mut_array_refs};
use solana_program::{
    clock::{Slot, DEFAULT_TICKS_PER_SECOND, DEFAULT_TICKS_PER_SLOT, SECONDS_PER_DAY},
    program_error::ProgramError,
    program_option::COption,
    pubkey::Pubkey,
};
use std::cmp::Ordering;

/// Number of slots to consider stale after
pub const STALE_AFTER_SLOTS: u64 = 10;

/// Last update state
#[derive(Clone, Debug, Default)]
pub struct LastUpdate {
    /// Last slot when updated
    pub slot: Slot,
}

impl LastUpdate {
    /// Create new last update
    pub fn new() -> Self {
        Self { slot: 0 }
    }

    /// Return slots elapsed since given slot
    pub fn slots_elapsed(&self, slot: Slot) -> Result<u64, ProgramError> {
        // @FIXME: what should happen if self.slot == 0?
        let slots_elapsed = slot
            .checked_sub(self.slot)
            .ok_or(LendingError::MathOverflow)?;
        Ok(slots_elapsed)
    }

    /// Set last update slot
    pub fn update_slot(&mut self, slot: Slot) {
        self.slot = slot;
    }

    // @FIXME: this will screw up interest rate tracking
    /// Set last update slot to 0
    pub fn mark_stale(&mut self) {
        self.update_slot(0);
    }

    /// Check if last update slot is too long ago
    pub fn is_stale(&self, slot: Slot) -> Result<bool, ProgramError> {
        Ok(self.slot == 0 || self.slots_elapsed(slot)? > STALE_AFTER_SLOTS)
    }
}

impl PartialEq for LastUpdate {
    fn eq(&self, other: &Self) -> bool {
        return &self.slot == &other.slot;
    }
}

impl PartialOrd for LastUpdate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.slot.partial_cmp(&other.slot)
    }
}
