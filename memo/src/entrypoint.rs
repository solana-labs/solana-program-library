//! Program entrypoint definitions

#![cfg(feature = "program")]
#![cfg(not(feature = "no-entrypoint"))]

use solana_sdk::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};
use std::str::from_utf8;

entrypoint!(process_instruction);
fn process_instruction<'a>(
    _program_id: &Pubkey,
    _accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    from_utf8(instruction_data).map_err(|_| ProgramError::InvalidInstructionData)?;
    Ok(())
}

// Pull in syscall stubs when building for non-BPF targets
#[cfg(not(target_arch = "bpf"))]
solana_sdk::program_stubs!();

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{program_error::ProgramError, pubkey::Pubkey};

    #[test]
    fn test_utf8_memo() {
        let program_id = Pubkey::new(&[0; 32]);

        let string = b"letters and such";
        assert_eq!(Ok(()), process_instruction(&program_id, &[], string));

        let emoji = "🐆".as_bytes();
        let bytes = [0xF0, 0x9F, 0x90, 0x86];
        assert_eq!(emoji, bytes);
        assert_eq!(Ok(()), process_instruction(&program_id, &[], &emoji));

        let mut bad_utf8 = bytes;
        bad_utf8[3] = 0xFF; // Invalid UTF-8 byte
        assert_eq!(
            Err(ProgramError::InvalidInstructionData),
            process_instruction(&program_id, &[], &bad_utf8)
        );
    }
}
