//! Offchain helper for fetching required accounts to build instructions

use {
    crate::{get_extra_account_metas_address, instruction::ExecuteInstruction},
    solana_sdk::{
        account::Account, instruction::AccountMeta, program_error::ProgramError, pubkey::Pubkey,
    },
    spl_tlv_account_resolution::state::ExtraAccountMetas,
    std::future::Future,
};

type AccountResult = Result<Option<Account>, AccountFetchError>;
type AccountFetchError = Box<dyn std::error::Error + Send + Sync>;

/// Offchain helper to get all additional required account metas for a mint
///
/// To be client-agnostic, this simply takes a function that will return a
/// `Future<Account>` for the given address. Can be called in the following way:
///
/// ```rust,ignore
/// use solana_client::nonblocking::rpc_client::RpcClient;
/// use solana_sdk::pubkey::Pubkey;
///
/// let program_id = Pubkey::new_unique();
/// let mint = Pubkey::new_unique();
/// let client = RpcClient::new_mock("succeeds".to_string());
///
/// let extra_account_metas = get_extra_account_metas(
///     |address| self.client.get_account(&address),
///     &program_id,
///     &mint,
/// ).await?;
/// ```
pub async fn get_extra_account_metas<F, Fut>(
    get_account_fn: F,
    permissioned_transfer_program_id: &Pubkey,
    mint: &Pubkey,
) -> Result<Vec<AccountMeta>, AccountFetchError>
where
    F: Fn(Pubkey) -> Fut,
    Fut: Future<Output = AccountResult>,
{
    let mut instruction_metas = vec![];
    let validation_address =
        get_extra_account_metas_address(mint, permissioned_transfer_program_id);
    let validation_account = get_account_fn(validation_address)
        .await?
        .ok_or(ProgramError::InvalidAccountData)?;
    ExtraAccountMetas::add_to_vec::<ExecuteInstruction>(
        &mut instruction_metas,
        &validation_account.data,
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
