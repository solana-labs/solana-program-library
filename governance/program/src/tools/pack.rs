//! General purpose packing utility functions

use arrayref::array_refs;
use solana_program::{program_error::ProgramError, program_option::COption, pubkey::Pubkey};

/// Unpacks COption from a slice
pub fn unpack_coption_pubkey(src: &[u8; 36]) -> Result<COption<Pubkey>, ProgramError> {
    let (tag, body) = array_refs![src, 4, 32];
    match *tag {
        [0, 0, 0, 0] => Ok(COption::None),
        [1, 0, 0, 0] => Ok(COption::Some(Pubkey::new_from_array(*body))),
        _ => Err(ProgramError::InvalidAccountData),
    }
}
