//! State helpers for working with the example program

use {
    solana_program::{instruction::AccountMeta, program_error::ProgramError, pubkey::Pubkey},
    spl_tlv_account_resolution::state::ExtraAccountMetas,
    spl_transfer_hook_interface::instruction::ExecuteInstruction,
};

/// Generate example data to be used directly in an account for testing
pub fn example_data() -> Result<Vec<u8>, ProgramError> {
    let account_metas = vec![
        AccountMeta {
            pubkey: Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
        },
        AccountMeta {
            pubkey: Pubkey::new_unique(),
            is_signer: false,
            is_writable: false,
        },
    ];
    let account_size = ExtraAccountMetas::size_of(account_metas.len())?;
    let mut data = vec![0; account_size];
    ExtraAccountMetas::init_with_account_metas::<ExecuteInstruction>(&mut data, &account_metas)?;
    Ok(data)
}
