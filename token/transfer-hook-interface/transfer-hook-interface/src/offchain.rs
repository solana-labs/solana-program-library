//! Offchain helper for fetching required accounts to build instructions

use {
    crate::{get_extra_account_metas_address, instruction::ExecuteInstruction},
    solana_program::{instruction::AccountMeta, program_error::ProgramError, pubkey::Pubkey},
    spl_tlv_account_resolution::state::ExtraAccountMetas,
    std::future::Future,
};

type AccountDataResult = Result<Option<Vec<u8>>, AccountFetchError>;
type AccountFetchError = Box<dyn std::error::Error + Send + Sync>;

/// Offchain helper to get all additional required account metas for a mint
///
/// To be client-agnostic and to avoid pulling in the full solana-sdk, this
/// simply takes a function that will return its data as `Future<Vec<u8>>` for
/// the given address. Can be called in the following way:
///
/// ```rust,ignore
/// use futures::future::TryFutureExt;
/// use solana_client::nonblocking::rpc_client::RpcClient;
/// use solana_program::pubkey::Pubkey;
///
/// let program_id = Pubkey::new_unique();
/// let mint = Pubkey::new_unique();
/// let client = RpcClient::new_mock("succeeds".to_string());
///
/// let extra_account_metas = get_extra_account_metas(
///     |address| self.client.get_account(&address).map_ok(|opt| opt.map(|acc| acc.data)),
///     &program_id,
///     &mint,
/// ).await?;
/// ```
pub async fn get_extra_account_metas<F, Fut>(
    get_account_data_fn: F,
    permissioned_transfer_program_id: &Pubkey,
    mint: &Pubkey,
) -> Result<Vec<AccountMeta>, AccountFetchError>
where
    F: Fn(Pubkey) -> Fut,
    Fut: Future<Output = AccountDataResult>,
{
    let mut instruction_metas = vec![];
    let validation_address =
        get_extra_account_metas_address(mint, permissioned_transfer_program_id);
    let validation_account_data = get_account_data_fn(validation_address)
        .await?
        .ok_or(ProgramError::InvalidAccountData)?;
    ExtraAccountMetas::add_to_vec::<ExecuteInstruction>(
        &mut instruction_metas,
        &validation_account_data,
    )?;
    instruction_metas.push(AccountMeta {
        pubkey: *permissioned_transfer_program_id,
        is_signer: false,
        is_writable: false,
    });
    instruction_metas.push(AccountMeta {
        pubkey: validation_address,
        is_signer: false,
        is_writable: false,
    });
    Ok(instruction_metas)
}
