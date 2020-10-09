//! State types

/// Lending pool state
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PoolState {}

/// Pool reserve state
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReserveState {}

/// Borrow obligation state
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ObligationState {}
