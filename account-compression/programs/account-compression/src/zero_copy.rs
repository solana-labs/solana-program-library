//! Implements ZeroCopy over ConcurrrentMerkleTree generics
use crate::error::error_msg;
use anchor_lang::prelude::*;
use bytemuck::Pod;
use spl_concurrent_merkle_tree::concurrent_merkle_tree::ConcurrentMerkleTree;
use std::mem::size_of;

pub trait ZeroCopy: Pod {
    fn load_mut_bytes<'a>(data: &'a mut [u8]) -> Result<&'a mut Self> {
        let size = size_of::<Self>();
        let data_len = data.len();

        Ok(bytemuck::try_from_bytes_mut(&mut data[..size])
            .map_err(error_msg::<Self>(data_len))
            .unwrap())
    }

    fn load_bytes<'a>(data: &'a [u8]) -> Result<&'a Self> {
        let size = size_of::<Self>();
        let data_len = data.len();

        Ok(bytemuck::try_from_bytes(&data[..size])
            .map_err(error_msg::<Self>(data_len))
            .unwrap())
    }
}

impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> ZeroCopy
    for ConcurrentMerkleTree<MAX_DEPTH, MAX_BUFFER_SIZE>
{
}
