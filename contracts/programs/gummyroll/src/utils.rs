use anchor_lang::{
    prelude::*,
    solana_program::{msg, program::invoke, program_error::ProgramError},
};
use crate::state::CandyWrapper;
use bytemuck::{Pod, PodCastError};
use concurrent_merkle_tree::merkle_roll::MerkleRoll;
use std::any::type_name;
use std::mem::size_of;

pub fn wrap_event<'info>(
    data: Vec<u8>,
    candy_wrapper_program: &Program<'info, CandyWrapper>,
) -> Result<()> {
    invoke(
        &candy_wrapper::wrap_instruction(data),
        &[candy_wrapper_program.to_account_info()],
    )?;
    Ok(())
}

pub trait ZeroCopy: Pod {
    fn load_mut_bytes<'a>(data: &'a mut [u8]) -> Result<&'a mut Self> {
        let size = size_of::<Self>();
        let data_len = data.len();

        Ok(bytemuck::try_from_bytes_mut(&mut data[..size])
            .map_err(error_msg::<Self>(data_len))
            .unwrap())
    }
}

impl<const MAX_DEPTH: usize, const MAX_BUFFER_SIZE: usize> ZeroCopy
    for MerkleRoll<MAX_DEPTH, MAX_BUFFER_SIZE>
{
}

pub fn error_msg<T>(data_len: usize) -> impl Fn(PodCastError) -> ProgramError {
    move |_: PodCastError| -> ProgramError {
        msg!(
            "Failed to load {}. Size is {}, expected {}",
            type_name::<T>(),
            data_len,
            size_of::<T>(),
        );
        ProgramError::InvalidAccountData
    }
}
