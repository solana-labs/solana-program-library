use std::slice::Iter;

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke,
    pubkey::Pubkey,
};

use super::{BaseState, BaseStateWithExtensions};

use {
    crate::{
        extension::{Extension, ExtensionType},
        pod::*,
    },
    bytemuck::{Pod, Zeroable},
};

/// Maximum number of additional accounts for a transfer authority
pub const MAX_ADDITIONAL_ACCOUNTS: usize = 3;

/// 8 byte instruction discriminator computed from hash("global:permissioned_token_transfer")
pub const PERMISSIONED_TOKEN_TRANSFER_INSTRUCTION_DATA: [u8; 8] =
    [86, 153, 189, 33, 202, 29, 46, 92];

/// Transfer authority extension data for mints.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PermissionedTransferMint {
    /// Program ID to CPI to on transfer
    pub program_id: OptionalNonZeroPubkey,
    /// Additional accounts required for transfer
    pub additional_accounts: [OptionalNonZeroPubkey; MAX_ADDITIONAL_ACCOUNTS],
}
impl Extension for PermissionedTransferMint {
    const TYPE: ExtensionType = ExtensionType::PermissionedTransferMint;
}

/// Transfer authority extension data for mints.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct PermissionedTransferAccount {}
impl Extension for PermissionedTransferAccount {
    const TYPE: ExtensionType = ExtensionType::PermissionedTransferAccount;
}

/// Call CPI to transfer authority to check if transfer is valid
pub fn permissioned_transfer_check<'info, S: BaseState, BSE: BaseStateWithExtensions<S>>(
    mint_state: &BSE,
    mint_info: &AccountInfo<'info>,
    source_account_info: &AccountInfo<'info>,
    destination_account_info: &AccountInfo<'info>,
    amount: u64,
    account_info_iter: &mut Iter<AccountInfo<'info>>,
) -> ProgramResult {
    if let Some(permissioned_transfer_mint) =
        mint_state.get_extension::<PermissionedTransferMint>().ok()
    {
        if let Some(program_id) = Option::<Pubkey>::from(permissioned_transfer_mint.program_id) {
            let mut account_metas = Vec::new();
            account_metas.push(AccountMeta::new(*mint_info.key, false));
            account_metas.push(AccountMeta::new(*source_account_info.key, false));
            account_metas.push(AccountMeta::new(*destination_account_info.key, false));

            let mut acount_infos = Vec::new();
            acount_infos.push(mint_info.clone());
            acount_infos.push(source_account_info.clone());
            acount_infos.push(destination_account_info.clone());

            for additional_account in permissioned_transfer_mint.additional_accounts.iter() {
                if let Some(pubkey) = Option::<Pubkey>::from(*additional_account) {
                    account_metas.push(AccountMeta::new(pubkey, false));
                    acount_infos.push(next_account_info(account_info_iter)?.clone());
                }
            }
            invoke(
                &Instruction {
                    program_id,
                    data: [
                        PERMISSIONED_TOKEN_TRANSFER_INSTRUCTION_DATA,
                        amount.to_le_bytes(),
                    ]
                    .concat(),
                    accounts: account_metas,
                },
                &acount_infos,
            )?;
        }
    }
    Ok(())
}
