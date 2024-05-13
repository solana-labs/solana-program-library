//! Anchor events are used to emit information necessary to
//! index changes made to a SPL ConcurrentMerkleTree

use anchor_lang::prelude::*;

mod application_data;
mod changelog_event;

pub use application_data::{ApplicationDataEvent, ApplicationDataEventV1};
pub use changelog_event::{ChangeLogEvent, ChangeLogEventV1};

#[derive(AnchorDeserialize, AnchorSerialize)]
#[repr(C)]
pub enum AccountCompressionEvent {
    ChangeLog(ChangeLogEvent),
    ApplicationData(ApplicationDataEvent),
}
