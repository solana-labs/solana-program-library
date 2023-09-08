//! todo doc

#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

// Make these available downstream for the macro to work without
// additional imports
pub use {
    serde,
    serde_with::{As, DisplayFromStr},
    spl_serde_derive::spl_serde,
};
