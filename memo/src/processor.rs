use crate::{error::MemoError, instruction::Instruction};
use solana_sdk::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult,
    program_error::PrintProgramError, pubkey::Pubkey,
};

entrypoint!(process_instruction);
fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Err(error) = process(program_id, accounts, instruction_data) {
        // catch the error so we can print it
        error.print::<MemoError>();
        return Err(error);
    }
    Ok(())
}

pub fn process<'a>(
    _program_id: &Pubkey,
    _accounts: &'a [AccountInfo<'a>],
    input: &[u8],
) -> ProgramResult {
    let _command = Instruction::deserialize(input)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{program_error::ProgramError, pubkey::Pubkey};

    #[test]
    fn test_utf8_memo() {
        let program_id = Pubkey::new(&[0; 32]);

        let string = "letters and such";
        let mut instruction_data = vec![0u8; string.len() + 1];
        let instruction = Instruction::Utf8(string);
        instruction.serialize(&mut instruction_data).unwrap();
        assert_eq!(Ok(()), process(&program_id, &mut vec![], &instruction_data));

        let emoji = "🐆";
        let bytes = [0x00, 0xF0, 0x9F, 0x90, 0x86];
        let mut instruction_data = vec![0u8; emoji.len() + 1];
        let instruction = Instruction::Utf8(emoji);
        instruction.serialize(&mut instruction_data).unwrap();
        assert_eq!(instruction_data, bytes);
        assert_eq!(Ok(()), process(&program_id, &mut vec![], &instruction_data));

        let mut bad_utf8 = bytes;
        bad_utf8[3] = 0xFF; // Invalid UTF-8 byte
        assert_eq!(
            Err(ProgramError::Custom(0)),
            process(&program_id, &mut vec![], &bad_utf8)
        );

        let mut bad_command_index = bytes;
        bad_command_index[0] = 0x01; // Invalid command index at index 0
        assert_eq!(
            Err(ProgramError::InvalidAccountData),
            process(&program_id, &mut vec![], &bad_command_index)
        );
    }
}
