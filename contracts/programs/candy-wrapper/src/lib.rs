use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint, entrypoint::ProgramResult,
    instruction::Instruction, pubkey::Pubkey,
};

declare_id!("WRAPYChf58WFCnyjXKJHtrPgzKXgHp6MD9aVDqJBbGh");

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(wrap);

pub fn wrap(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    Ok(())
}

pub fn wrap_instruction(data: Vec<u8>) -> Instruction {
    Instruction {
        program_id: crate::id(),
        accounts: vec![],
        data,
    }
}
