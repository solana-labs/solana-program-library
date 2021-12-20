#![allow(missing_docs)]
/// Derived from https://github.com/project-serum/anchor/blob/9224e0fa99093943a6190e396bccbc3387e5b230/examples/pyth/programs/pyth/src/pc.rs
use bytemuck::{
    cast_slice, cast_slice_mut, from_bytes, from_bytes_mut, try_cast_slice, try_cast_slice_mut,
    Pod, PodCastError, Zeroable,
};
use std::mem::size_of;

pub const MAGIC: u32 = 0xa1b2c3d4;
pub const VERSION_2: u32 = 2;
pub const VERSION: u32 = VERSION_2;
pub const MAP_TABLE_SIZE: usize = 640;
pub const PROD_ACCT_SIZE: usize = 512;
pub const PROD_HDR_SIZE: usize = 48;
pub const PROD_ATTR_SIZE: usize = PROD_ACCT_SIZE - PROD_HDR_SIZE;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct AccKey {
    pub val: [u8; 32],
}

#[derive(PartialEq, Copy, Clone)]
#[repr(C)]
pub enum AccountType {
    Unknown,
    Mapping,
    Product,
    Price,
}

#[derive(PartialEq, Copy, Clone)]
#[repr(C)]
pub enum PriceStatus {
    Unknown,
    Trading,
    Halted,
    Auction,
}

#[derive(PartialEq, Copy, Clone)]
#[repr(C)]
pub enum CorpAction {
    NoCorpAct,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PriceInfo {
    pub price: i64,
    pub conf: u64,
    pub status: PriceStatus,
    pub corp_act: CorpAction,
    pub pub_slot: u64,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PriceComp {
    publisher: AccKey,
    agg: PriceInfo,
    latest: PriceInfo,
}

#[derive(PartialEq, Copy, Clone)]
#[repr(C)]
pub enum PriceType {
    Unknown,
    Price,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Price {
    pub magic: u32,       // pyth magic number
    pub ver: u32,         // program version
    pub atype: u32,       // account type
    pub size: u32,        // price account size
    pub ptype: PriceType, // price or calculation type
    pub expo: i32,        // price exponent
    pub num: u32,         // number of component prices
    pub unused: u32,
    pub curr_slot: u64,        // currently accumulating price slot
    pub valid_slot: u64,       // valid slot-time of agg. price
    pub twap: i64,             // time-weighted average price
    pub avol: u64,             // annualized price volatility
    pub drv0: i64,             // space for future derived values
    pub drv1: i64,             // space for future derived values
    pub drv2: i64,             // space for future derived values
    pub drv3: i64,             // space for future derived values
    pub drv4: i64,             // space for future derived values
    pub drv5: i64,             // space for future derived values
    pub prod: AccKey,          // product account key
    pub next: AccKey,          // next Price account in linked list
    pub agg_pub: AccKey,       // quoter who computed last aggregate price
    pub agg: PriceInfo,        // aggregate price info
    pub comp: [PriceComp; 32], // price components one per quoter
}

#[cfg(target_endian = "little")]
unsafe impl Zeroable for Price {}

#[cfg(target_endian = "little")]
unsafe impl Pod for Price {}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Product {
    pub magic: u32,                 // pyth magic number
    pub ver: u32,                   // program version
    pub atype: u32,                 // account type
    pub size: u32,                  // price account size
    pub px_acc: AccKey,             // first price account in list
    pub attr: [u8; PROD_ATTR_SIZE], // key/value pairs of reference attr.
}

#[cfg(target_endian = "little")]
unsafe impl Zeroable for Product {}

#[cfg(target_endian = "little")]
unsafe impl Pod for Product {}

pub fn load<T: Pod>(data: &[u8]) -> Result<&T, PodCastError> {
    let size = size_of::<T>();
    Ok(from_bytes(cast_slice::<u8, u8>(try_cast_slice(
        &data[0..size],
    )?)))
}

pub fn load_mut<T: Pod>(data: &mut [u8]) -> Result<&mut T, PodCastError> {
    let size = size_of::<T>();
    Ok(from_bytes_mut(cast_slice_mut::<u8, u8>(
        try_cast_slice_mut(&mut data[0..size])?,
    )))
}
