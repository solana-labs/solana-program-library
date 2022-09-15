//! Program instructions

use {
    crate::get_token_upgrade_authority_address,
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
    },
};

/// Instructions supported by the TokenUpgrade program
#[derive(Clone, Copy, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum TokenUpgradeInstruction {
    /// Burns all of the original tokens in the user's account, and transfers the same
    /// amount of tokens from an account owned by a PDA into another account.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable]` Original token account to burn from
    ///   1. `[writeable]` Original token mint
    ///   2. `[writeable]` Escrow of new tokens held by or delegated to PDA at address:
    ///       `get_token_upgrade_authority_address(original_mint, new_mint, program_id)`
    ///   3. `[writeable]` New token account to transfer into
    ///   4. `[]` New token mint
    ///   5. `[]` Transfer authority (owner or delegate) of new token escrow held by PDA, must be:
    ///       `get_token_upgrade_authority_address(original_mint, new_mint, program_id)`
    ///   6. `[]` SPL Token program for original mint
    ///   7. `[]` SPL Token program for new mint
    ///   8. `[]` Original token account transfer authority (owner or delegate)
    ///   9. ..9+M `[signer]` M multisig signer accounts
    ///
    /// Data expected by this instruction:
    ///   None
    ///
    Exchange,
}

/// Create an `Exchange` instruction
#[allow(clippy::too_many_arguments)]
pub fn exchange(
    program_id: &Pubkey,
    original_account: &Pubkey,
    original_mint: &Pubkey,
    new_escrow: &Pubkey,
    new_account: &Pubkey,
    new_mint: &Pubkey,
    original_token_program_id: &Pubkey,
    new_token_program_id: &Pubkey,
    original_transfer_authority: &Pubkey,
    original_multisig_signers: &[&Pubkey],
) -> Instruction {
    let escrow_authority = get_token_upgrade_authority_address(original_mint, new_mint, program_id);
    let mut accounts = Vec::with_capacity(9usize.saturating_add(original_multisig_signers.len()));
    accounts.push(AccountMeta::new(*original_account, false));
    accounts.push(AccountMeta::new(*original_mint, false));
    accounts.push(AccountMeta::new(*new_escrow, false));
    accounts.push(AccountMeta::new(*new_account, false));
    accounts.push(AccountMeta::new(*new_mint, false));
    accounts.push(AccountMeta::new_readonly(escrow_authority, false));
    accounts.push(AccountMeta::new_readonly(*original_token_program_id, false));
    accounts.push(AccountMeta::new_readonly(*new_token_program_id, false));
    accounts.push(AccountMeta::new_readonly(
        *original_transfer_authority,
        original_multisig_signers.is_empty(),
    ));
    for signer_pubkey in original_multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**signer_pubkey, true));
    }

    Instruction {
        program_id: *program_id,
        accounts,
        data: vec![TokenUpgradeInstruction::Exchange.into()],
    }
}
