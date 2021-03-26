//! Program instruction processor

use {
    solana_program::{
        account_info::AccountInfo,
        borsh::get_packed_len,
        entrypoint::ProgramResult,
        pubkey::Pubkey,
    },
    borsh::BorshSchema,
};

/// Instruction processor
pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _instruction_data: &[u8],
) -> ProgramResult {
    // Try to get packed len in BPF
    let _len = get_packed_len::<MyStruct>();

    Ok(())
}

#[derive(BorshSchema)]
struct MyStruct {
    /// Some data
    pub field: u8,
}
