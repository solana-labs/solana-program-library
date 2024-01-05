//! Offchain helper for fetching required accounts to build instructions

pub use spl_transfer_hook_interface::offchain::{AccountDataResult, AccountFetchError};
use {
    crate::{
        error::TokenError,
        extension::{transfer_hook, StateWithExtensions},
        state::Mint,
    },
    solana_program::{instruction::Instruction, msg, program_error::ProgramError, pubkey::Pubkey},
    spl_transfer_hook_interface::{
        get_extra_account_metas_address, offchain::resolve_extra_account_metas,
    },
    std::future::Future,
};

/// Offchain helper to get all additional required account metas for a checked
/// transfer
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
/// Note that this offchain helper will build a new `Execute` instruction,
/// resolve the extra account metas, and then add them to the transfer
/// instruction. This is because the extra account metas are configured
/// specifically for the `Execute` instruction, which requires five accounts
/// (source, mint, destination, authority, and validation state), wheras the
/// transfer instruction only requires four (source, mint, destination, and
/// authority) in addition to `n` number of multisig authorities.
pub async fn resolve_extra_transfer_account_metas<F, Fut>(
    instruction: &mut Instruction,
    fetch_account_data_fn: F,
    mint_address: &Pubkey,
    amount: u64,
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
        // Convert the transfer instruction into an `Execute` instruction,
        // then resolve the extra account metas as configured in the validation
        // account data, then finally add the extra account metas to the original
        // transfer instruction.
        if instruction.accounts.len() < 4 {
            msg!("Not a valid transfer instruction");
            Err(TokenError::InvalidInstruction)?;
        }

        let mut execute_ix = spl_transfer_hook_interface::instruction::execute(
            &program_id,
            &instruction.accounts[0].pubkey,
            &instruction.accounts[1].pubkey,
            &instruction.accounts[2].pubkey,
            &instruction.accounts[3].pubkey,
            &get_extra_account_metas_address(mint_address, &program_id),
            amount,
        );

        resolve_extra_account_metas(
            &mut execute_ix,
            fetch_account_data_fn,
            mint_address,
            &program_id,
        )
        .await?;

        instruction
            .accounts
            .extend_from_slice(&execute_ix.accounts[5..]);
    }
    Ok(())
}
