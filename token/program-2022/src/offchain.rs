//! Offchain helper for fetching required accounts to build instructions

pub use spl_transfer_hook_interface::offchain::{AccountDataResult, AccountFetchError};
use {
    crate::{
        extension::{transfer_hook, StateWithExtensions},
        state::Mint,
    },
    solana_program::{instruction::Instruction, program_error::ProgramError, pubkey::Pubkey},
    spl_transfer_hook_interface::offchain::resolve_extra_account_metas,
    std::future::Future,
};

/// Offchain helper to get all additional required account metas for a checked transfer
///
/// To be client-agnostic and to avoid pulling in the full solana-sdk, this
/// simply takes a function that will return its data as `Future<Vec<u8>>` for
/// the given address. Can be called in the following way:
///
/// ```rust,ignore
/// use futures_util::TryFutureExt;
/// use solana_client::nonblocking::rpc_client::RpcClient;
/// use solana_program::pubkey::Pubkey;
///
/// let mint = Pubkey::new_unique();
/// let client = RpcClient::new_mock("succeeds".to_string());
/// let mut account_metas = vec![];
///
/// get_extra_transfer_account_metas(
///     &mut account_metas,
///     |address| self.client.get_account(&address).map_ok(|opt| opt.map(|acc| acc.data)),
///     &mint,
/// ).await?;
/// ```
pub async fn resolve_extra_transfer_account_metas<F, Fut>(
    instruction: &mut Instruction,
    fetch_account_data_fn: F,
    mint_address: &Pubkey,
) -> Result<(), AccountFetchError>
where
    F: Fn(Pubkey) -> Fut,
    Fut: Future<Output = AccountDataResult>,
{
    let mint_data = fetch_account_data_fn(*mint_address)
        .await?
        .ok_or(ProgramError::InvalidAccountData)?;
    let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;
    if let Some(program_id) = transfer_hook::get_program_id(&mint) {
        resolve_extra_account_metas(
            instruction,
            fetch_account_data_fn,
            mint_address,
            &program_id,
        )
        .await?;
    }
    Ok(())
}
