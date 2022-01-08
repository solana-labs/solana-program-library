use {
    super::instruction::*,
    solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey},
};

/// TODO: inline `zk_token_program::processor.rs` here
pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    match decode_instruction_type(input)? {
        ConfidentialTransferInstruction::Todo => {
            let todo_data = decode_instruction_data::<()>(input)?;
            msg!("Todo: {:?}", todo_data);
            Ok(())
        }
    }
}
