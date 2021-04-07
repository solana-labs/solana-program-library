use {
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
    pub allow_further_share_creation: bool,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct AddTokenToInactivatedFractionalizedTokenPoolArgs {
    pub amount: u64,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct ActivateFractionalizedTokenPoolArgs {
    pub number_of_shares: u64,
}

/// Instructions supported by the Fraction program.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum FractionInstruction {
    /// Initialize a fractionalized token pool, starts inactivate. Add tokens in subsequent instructions, then activate.
    ///   0. `[writable]` Initialized fractional share mint with 0 tokens in supply
    ///   1. `[writable]` Initialized redeem treasury token account with 0 tokens in supply
    ///   2. `[writable]` Initialized fraction treasury token account with 0 tokens in supply
    ///   3. `[writable]` Uninitialized fractionalized token ledger account
    ///   4. `[]` Authority
    ///   5. `[]` Pricing Lookup Address
    ///   6. `[]` Token program
    ///   7. `[]` Rent sysvar
    InitFractionalizedTokenPool(InitFractionalizedTokenPoolArgs),

    /// Add a token to a inactivate fractionalized token pool
    ///   0. `[writable]` Uninitialized Token Fractional Registry account address (will be created and allocated by this endpoint)
    ///                   Address should be pda with seed of [PREFIX, fractional_token_ledger_address, token_mint_address]
    ///   1. `[writable]` Initialized Token account
    ///   2. `[writable]` Initialized Token vault account with authority of this program
    ///   3. `[writable]` Initialized inactivate fractionalized token pool
    ///   4. `[signer]` Payer
    ///   5. `[]` Transfer Authority to move desired token amount from token account to vault
    ///   6. `[]` Token program
    ///   7. `[]` Rent sysvar
    ///   8. `[]` System account sysvar
    AddTokenToInactivatedFractionalizedTokenPool(AddTokenToInactivatedFractionalizedTokenPoolArgs),

    ///   0. `[writable]` Initialized inactivated fractionalized token pool
    ///   1. `[writable]` Fraction mint
    ///   2. `[writable]` Fraction treasury
    ///   3. `[]` Fraction mint authority for the program
    ///   4. `[]` Token program
    ActivateFractionalizedTokenPool(ActivateFractionalizedTokenPoolArgs),

    ///   0. `[writable]` Initialized activated fractionalized token pool
    ///   1. `[writable]` Token account containing your portion of the outstanding fraction shares
    ///   1. `[writable]` Token account of the redeem_treasury mint type that you will pay with
    ///   1. `[writable]` Fraction mint
    ///   1. `[writable]` Redeem treasury account
    ///   1. `[]` Transfer authority for the  token account that you will pay with
    ///   1. `[]` Burn authority for the fraction token account containing your outstanding fraction shares
    ///   1. `[]` External pricing lookup address
    ///   4. `[]` Token program
    CombineFractionalizedTokenPool,
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
