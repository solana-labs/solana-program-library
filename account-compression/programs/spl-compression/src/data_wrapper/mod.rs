use anchor_lang::{
    prelude::*,
    solana_program::program::invoke,
};

#[derive(Clone)]
pub struct Wrapper;

impl anchor_lang::Id for Wrapper {
    fn id() -> Pubkey {
        wrapper::id()
    }
}

pub fn wrap_event<'info>(
    data: Vec<u8>,
    candy_wrapper_program: &Program<'info, Wrapper>,
) -> Result<()> {
    invoke(
        &wrapper::wrap_instruction(data),
        &[candy_wrapper_program.to_account_info()],
    )?;
    Ok(())
}
