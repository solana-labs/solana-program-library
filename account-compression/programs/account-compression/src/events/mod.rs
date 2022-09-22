//! Anchor events are used to emit information necessary to
//! index changes made to a SPL ConcurrentMerkleTree

use anchor_lang::prelude::*;

mod changelog_event;

pub use changelog_event::ChangeLogEvent;

#[derive(AnchorDeserialize, AnchorSerialize)]
#[repr(C)]
pub enum AccountCompressionEvent {
    ChangeLog(ChangeLogEvent),
}
