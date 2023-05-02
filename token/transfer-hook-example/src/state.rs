//! State helpers for working with the example program

use {
    solana_program::{instruction::AccountMeta, program_error::ProgramError},
    spl_tlv_account_resolution::state::ExtraAccountMetas,
    spl_transfer_hook_interface::instruction::ExecuteInstruction,
};

/// Generate example data to be used directly in an account for testing
pub fn example_data(account_metas: &[AccountMeta]) -> Result<Vec<u8>, ProgramError> {
    let account_size = ExtraAccountMetas::size_of(account_metas.len())?;
    let mut data = vec![0; account_size];
    ExtraAccountMetas::init_with_account_metas::<ExecuteInstruction>(&mut data, account_metas)?;
    Ok(data)
}
