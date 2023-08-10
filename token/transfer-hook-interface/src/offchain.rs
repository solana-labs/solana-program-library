//! Offchain helper for fetching required accounts to build instructions

use {
    crate::{get_extra_account_metas_address, instruction::ExecuteInstruction},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_tlv_account_resolution::state::ExtraAccountMetaList,
    std::future::Future,
};

/// Type representing the output of an account fetching function, for easy
/// chaining between APIs
pub type AccountDataResult = Result<Option<Vec<u8>>, AccountFetchError>;
/// Generic error type that can come out of any client while fetching account data
pub type AccountFetchError = Box<dyn std::error::Error + Send + Sync>;

/// Offchain helper to get all additional required account metas for a mint
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
/// let program_id = Pubkey::new_unique();
/// let mint = Pubkey::new_unique();
/// let client = RpcClient::new_mock("succeeds".to_string());
/// let mut account_metas = vec![];
///
/// get_extra_account_metas(
///     &mut account_metas,
///     |address| self.client.get_account(&address).map_ok(|opt| opt.map(|acc| acc.data)),
///     &mint,
///     &program_id,
/// ).await?;
/// ```
pub async fn resolve_extra_account_metas<F, Fut>(
    instruction: &mut Instruction,
    get_account_data_fn: F,
    mint: &Pubkey,
    permissioned_transfer_program_id: &Pubkey,
) -> Result<(), AccountFetchError>
where
    F: Fn(Pubkey) -> Fut,
    Fut: Future<Output = AccountDataResult>,
{
    let validation_address =
        get_extra_account_metas_address(mint, permissioned_transfer_program_id);
    let validation_account_data = get_account_data_fn(validation_address)
        .await?
        .ok_or(ProgramError::InvalidAccountData)?;
    ExtraAccountMetaList::add_to_instruction::<ExecuteInstruction>(
        instruction,
        &validation_account_data,
    )?;
    // The onchain helpers pull out the required accounts from an opaque
    // slice by pubkey, so the order doesn't matter here!
    instruction.accounts.push(AccountMeta::new_readonly(
        *permissioned_transfer_program_id,
        false,
    ));
    instruction
        .accounts
        .push(AccountMeta::new_readonly(validation_address, false));

    Ok(())
}
