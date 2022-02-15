//! Large uint types

// required for clippy
#![allow(clippy::assign_op_pattern)]
#![allow(clippy::ptr_offset_with_cast)]
#![allow(clippy::manual_range_contains)]

use uint::construct_uint;

construct_uint! {
    pub struct U256(4);
}
construct_uint! {
    pub struct U192(3);
}
