use {
    crate::state::PricingLookupType,
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        sysvar,
    },
};

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct InitFractionalizedTokenPoolArgs {
    pub allow_share_redemption: bool,
    pub pricing_lookup_type: PricingLookupType,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct AddTokenToInactivatedFractionalizedTokenPoolArgs {
    pub amount: u64,
}

/// Instructions supported by the Fraction program.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum FractionInstruction {
    /// Initialize a fractionalized token pool, starts deactivated. Add tokens in subsequent instructions, then activate.
    ///   0. `[writable]` Initialized fractional share mint with 0 tokens in supply
    ///   1. `[writable]` Initialized treasury token account with 0 tokens in supply
    ///   2. `[writable]` Uninitialized fractionalized token ledger account
    ///   4. `[]` Authority
    ///   5. `[]` Pricing Lookup Address
    InitFractionalizedTokenPool(InitFractionalizedTokenPoolArgs),

    /// Add a token to a deactivated fractionalized token pool
    ///   0. `[writable]` Uninitialized Token Fractional Registry account address (will be created and allocated by this endpoint)
    ///                   Address should be pda with seed of [PREFIX, fractional_token_ledger_address, token_mint_address]
    ///   1. `[writable]` Initialized Token account
    ///   2. `[writable]` Initialized Token vault account with authority of this program
    ///   3. `[writable]` Initialized deactivated fractionalized token pool
    ///   4. `[]` Payer
    ///   5. `[]` Transfer Authority to move desired token amount from token account to vault
    ///   6. `[]` Token program
    ///   7. `[]` Rent sysvar
    ///   8. `[]` System account sysvar
    AddTokenToInactivatedFractionalizedTokenPool(AddTokenToInactivatedFractionalizedTokenPoolArgs),
}
/*
/// Creates an CreateFractionAccounts instruction
#[allow(clippy::too_many_arguments)]
pub fn create_metadata_accounts(
    program_id: Pubkey,
    name_symbol_account: Pubkey,
    metadata_account: Pubkey,
    mint: Pubkey,
    mint_authority: Pubkey,
    payer: Pubkey,
    update_authority: Pubkey,
    name: String,
    symbol: String,
    uri: String,
    allow_duplication: bool,
    update_authority_is_signer: bool,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(name_symbol_account, false),
            AccountMeta::new(metadata_account, false),
            AccountMeta::new_readonly(mint, false),
            AccountMeta::new_readonly(mint_authority, true),
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new_readonly(update_authority, update_authority_is_signer),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: FractionInstruction::CreateFractionAccounts(CreateFractionAccountArgs {
            data: Data { name, symbol, uri },
            allow_duplication,
        })
        .try_to_vec()
        .unwrap(),
    }
}
*/
