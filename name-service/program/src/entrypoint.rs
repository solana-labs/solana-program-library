use {
    crate::error::NameServiceError,
    crate::processor::Processor,
    num_traits::FromPrimitive,
    solana_program::{
        account_info::AccountInfo, decode_error::DecodeError, entrypoint,
        entrypoint::ProgramResult, msg, program_error::PrintProgramError, pubkey::Pubkey,
    },
};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Entrypoint");
    if let Err(error) = Processor::process_instruction(program_id, accounts, instruction_data) {
        // catch the error so we can print it
        error.print::<NameServiceError>();
        return Err(error);
    }
    Ok(())
}

impl PrintProgramError for NameServiceError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            NameServiceError::OutOfSpace => msg!("Error: Registry is out of space!"),
        }
    }
}
