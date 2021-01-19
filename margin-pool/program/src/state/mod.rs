//! State transition types
mod pool;
mod position;

pub use pool::*;
pub use position::*;

pub const UNINITIALIZED_VERSION: u8 = 0;
