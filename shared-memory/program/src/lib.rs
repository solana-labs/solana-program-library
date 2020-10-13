#![deny(missing_docs)]

//! Shared memory program for the Solana blockchain.
//
// Useful for returning data from cross-program invoked programs to the invoker.
//
// This program is highly optimized for its particular use case and does not
// implement the typical `process_instruction` entrypoint.

use arrayref::{array_refs, mut_array_refs};
use solana_sdk::{
    entrypoint::MAX_PERMITTED_DATA_INCREASE, entrypoint::SUCCESS, program_error::ProgramError,
    pubkey::Pubkey,
};
use std::{
    mem::{align_of, size_of},
    ptr::read,
    slice::{from_raw_parts, from_raw_parts_mut},
};

// TODO solana_sdk::declare_id!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

/// A more efficient `copy_from_slice` implementation.
fn fast_copy(mut src: &[u8], mut dst: &mut [u8]) {
    while src.len() >= 8 {
        #[allow(clippy::ptr_offset_with_cast)]
        let (src_word, src_rem) = array_refs![src, 8; ..;];
        #[allow(clippy::ptr_offset_with_cast)]
        let (dst_word, dst_rem) = mut_array_refs![dst, 8; ..;];
        *dst_word = *src_word;
        src = src_rem;
        dst = dst_rem;
    }
    dst.copy_from_slice(src);
}

/// Deserializes only the particular input parameters that the shared memory
/// program uses.  For more information about the format of the serialized input
/// parameters see `solana_sdk::entrypoint::deserialize`
unsafe fn deserialize_input_parametes<'a>(input: *mut u8) -> Result<(&'a mut [u8], &'a [u8]), u64> {
    // Only one account expected
    let num_accounts = read(input as *const u64);
    if num_accounts == 0 {
        return Err(ProgramError::NotEnoughAccountKeys.into());
    } else if num_accounts > 1 {
        return Err(ProgramError::InvalidArgument.into());
    }

    // Offset to the first (and only) account's data length
    let data_len_offset = size_of::<u64>()
        + size_of::<u8>()
        + size_of::<u8>()
        + size_of::<u8>()
        + size_of::<u8>()
        + size_of::<u32>()
        + size_of::<Pubkey>()
        + size_of::<Pubkey>()
        + size_of::<u64>();

    let account_data_len = read(input.add(data_len_offset) as *const usize);
    let data_ptr = input.add(data_len_offset + size_of::<u64>());
    let account_data = from_raw_parts_mut(data_ptr, account_data_len);

    // Offset from the account data pointer to the instruction's data length
    let instruction_len_offset = account_data_len
        + MAX_PERMITTED_DATA_INCREASE
        + (account_data_len as *const u8).align_offset(align_of::<u128>())
        + size_of::<u64>();

    let instruction_data_len = read(data_ptr.add(instruction_len_offset) as *const usize);
    let instruction_data = from_raw_parts(
        data_ptr.add(instruction_len_offset + size_of::<u64>()),
        instruction_data_len,
    );

    Ok((account_data, instruction_data))
}

/// This program uses the raw Solana runtime's entrypoint which takes a pointer
/// to serialized input parameters.  For more information about the format of
/// the serialized input parameters see `solana_sdk::entrypoint::deserialize`
///
/// This program expects one account and writes all the instruction data into
/// the account's data starting at offset 0.
/// # Safety
#[no_mangle]
pub unsafe extern "C" fn entrypoint(input: *mut u8) -> u64 {
    match deserialize_input_parametes(input) {
        Ok((account_data, instruction_data)) => {
            match account_data.get_mut(..instruction_data.len()) {
                None => return ProgramError::AccountDataTooSmall.into(),
                Some(data) => fast_copy(instruction_data, data),
            };
        }
        Err(err) => return err,
    }
    SUCCESS
}
