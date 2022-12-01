use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{pubkey::Pubkey, entrypoint::ProgramResult};

use crate::error::MultiDelegateError;


#[derive(BorshSerialize, BorshDeserialize)]
pub struct Delegate {
    authority: Pubkey,
    amount: u64,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct MultiDelegate {
    delegates: Vec<Delegate>
}

impl MultiDelegate {
    pub const DEFAULT_LEN: usize = 200;

    pub fn approve(&mut self, authority: &Pubkey, amount: u64) {
        if let Some(delegate) = self.delegates.iter_mut().find(|delegate| &delegate.authority == authority) {
            delegate.amount = amount;
        } else {
            self.delegates.push(Delegate {
                authority: *authority,
                amount,
            })
        }
    }

    pub fn revoke(&mut self, authority: &Pubkey) {
        if let Some(position) = self.delegates.iter().position(|delegate| &delegate.authority == authority) {
            self.delegates.swap_remove(position);
        } else {
            // Cannot revoke error or no-op?
        }
    }

    pub fn transfer_with_delegate(&mut self, authority: &Pubkey, amount: u64) -> ProgramResult {
        for mut delegate in self.delegates.iter_mut() {
            if &delegate.authority == authority {
                delegate.amount = delegate.amount.checked_sub(amount)
                    .ok_or(MultiDelegateError::InsufficientDelegateAmount)?;
                return Ok(());
            }
        }

        Err(MultiDelegateError::DelegateNotFound.into())
    }
}