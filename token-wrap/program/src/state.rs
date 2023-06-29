//! Program state

use {
    bytemuck::{Pod, Zeroable},
    solana_program::pubkey::Pubkey,
};

/// Backpointer
///
/// Since the backpointer account address is derived from the wrapped mint, it
/// allows clients to easily work with wrapped tokens.
///
/// Try to fetch the account at `get_wrapped_mint_backpointer_address`.
///  * if it doesn't exist, then the token is not wrapped
///  * if it exists, read the data in the account as the unwrapped mint address
///
/// With this info, clients can easily unwrap tokens, even if they don't know
/// the origin.
#[derive(Copy, Clone, Debug, PartialEq, Pod, Zeroable)]
#[repr(transparent)]
pub struct Backpointer {
    /// Address that the wrapped mint is wrapping
    pub unwrapped_mint: Pubkey,
}
