//! Program instructions

use crate::get_multi_delegate_address;

use {
    crate::id,
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
};

/// Instructions supported by the AssociatedTokenAccount program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum MultiDelegateInstruction {
    ///   0. `[writeable,signer]` Funding account (must be a system account)
    ///   1. `[]` Token account address
    ///   2. `[]` Token account owner
    ///   3. `[writeable]` Multi delegate
    ///   4. `[]` System program
    ///   5. `[]` SPL Token program
    Create,
    ///   0. `[writeable,signer]` Token account owner
    ///   1. `[writeable]` Token account
    ///   2. `[writeable]` Multi delegate
    ///   3. `[]` delegate to be added/edited in multi delegate
    ///   4. `[]` System program
    ///   5. `[]` SPL Token program
    Approve { amount: u64 },
    ///   0. `[writeable,signer]` Token account owner
    ///   1. `[writeable]` Token account
    ///   2. `[writeable]` Multi delegate
    ///   3. `[]` delegate to be revoked in multi delegate
    Revoke,
    ///   0. `[signer]` Delegate
    ///   2. `[writeable]` Multi delegate
    ///   3. `[writeable]` Source token account
    ///   4. `[writeable]` Destination token account
    ///   5. `[]` SPL Token program
    Transfer { amount: u64 },
    Close,
}

pub fn create_multi_delegate(
    funding_address: &Pubkey,
    token_account_owner_address: &Pubkey,
    token_account_address: &Pubkey,
) -> Instruction {
    let multi_delegate_address =
        get_multi_delegate_address(token_account_owner_address, token_account_address);

    Instruction {
        program_id: id(),
        accounts: vec![
            AccountMeta::new(*funding_address, true),
            AccountMeta::new_readonly(*token_account_owner_address, true),
            AccountMeta::new_readonly(*token_account_address, false),
            AccountMeta::new_readonly(multi_delegate_address, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],
        data: MultiDelegateInstruction::Create.try_to_vec().unwrap(),
    }
}
