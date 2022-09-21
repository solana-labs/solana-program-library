//! Anchor events are used to emit information necessary to
//! index changes made to a SPL ConcurrentMerkleTree
mod changelog_event;

pub use changelog_event::ChangeLogEvent;
