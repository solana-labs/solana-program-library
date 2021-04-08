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
pub struct InitVaultArgs {
    pub allow_further_share_creation: bool,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct AddTokenToInactiveVaultArgs {
    pub amount: u64,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub struct NumberOfShareArgs {
    pub number_of_shares: u64,
}

/// Instructions supported by the Fraction program.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum VaultInstruction {
    /// Initialize a token vault, starts inactivate. Add tokens in subsequent instructions, then activate.
    ///   0. `[writable]` Initialized fractional share mint with 0 tokens in supply
    ///   1. `[writable]` Initialized redeem treasury token account with 0 tokens in supply
    ///   2. `[writable]` Initialized fraction treasury token account with 0 tokens in supply
    ///   3. `[writable]` Uninitialized fractionalized token ledger account
    ///   4. `[]` Authority on the vault
    ///   5. `[]` Pricing Lookup Address
    ///   6. `[]` Token program
    ///   7. `[]` Rent sysvar
    InitVault(InitVaultArgs),

    /// Add a token to a inactive token vault
    ///   0. `[writable]` Uninitialized Token Fractional Registry account address (will be created and allocated by this endpoint)
    ///                   Address should be pda with seed of [PREFIX, fractional_token_ledger_address, token_mint_address]
    ///   1. `[writable]` Initialized Token account
    ///   2. `[writable]` Initialized Token safety deposit box account with authority of this program
    ///   3. `[writable]` Initialized inactive fractionalized token vault
    ///   4. `[signer]` Authority on the vault
    ///   5. `[signer]` Payer
    ///   6. `[]` Transfer Authority to move desired token amount from token account to safety deposit
    ///   7. `[]` Token program
    ///   8. `[]` Rent sysvar
    ///   9. `[]` System account sysvar
    AddTokenToInactiveVault(AddTokenToInactiveVaultArgs),

    /// Activates the vault, distributing initial shares into the fraction treasury.
    /// Tokens can no longer be removed in this state until Combination.
    ///   0. `[writable]` Initialized inactivated fractionalized token vault
    ///   1. `[writable]` Fraction mint
    ///   2. `[writable]` Fraction treasury
    ///   3. `[signer]` Authority on the vault
    ///   4. `[]` Fraction mint authority for the program
    ///   5. `[]` Token program
    ActivateVault(NumberOfShareArgs),

    /// This act checks the external pricing oracle for permission to combine and the price of the circulating market cap to do so.
    /// If you can afford it, this amount is charged and placed into the redeem treasury for shareholders to redeem at a later time.
    /// The treasury then unlocks into Combine state and you can remove the tokens.
    ///   0. `[writable]` Initialized activated token vault
    ///   1. `[writable]` Token account containing your portion of the outstanding fraction shares
    ///   2. `[writable]` Token account of the redeem_treasury mint type that you will pay with
    ///   3. `[writable]` Fraction mint
    ///   4. `[writable]` Fraction treasury account
    ///   5. `[writable]` Redeem treasury account
    ///   6. `[signer]` Authority on the vault
    ///   7. `[]` Transfer authority for the  token account that you will pay with
    ///   8. `[]` Burn authority for the fraction token account containing your outstanding fraction shares
    ///   9. `[]` PDA-based Burn authority for the fraction treasury account containing the uncirculated shares seed [PREFIX, program_id]
    ///   10. `[]` External pricing lookup address
    ///   11. `[]` Token program
    CombineVault,

    /// If in the combine state, shareholders can hit this endpoint to burn shares in exchange for monies from the treasury.
    /// Once fractional supply is zero and all tokens have been removed this action will take vault to Deactivated
    ///   0. `[writable]` Initialized Token account containing your fractional shares
    ///   1. `[writable]` Initialized Destination token account where you wish your proceeds to arrive
    ///   2. `[writable]` Fraction mint
    ///   3. `[writable]` Redeem treasury account
    ///   4. `[]` Transfer authority for the transfer of proceeds from redeem treasury to destination
    ///   5. `[]` Burn authority for the burning of all your fractional shares
    ///   6. `[]` Combined token vault
    ///   7. `[]` Token program
    ///   8. `[]` Rent sysvar
    RedeemShares,

    /// If in combine state, authority on vault can hit this to withdrawal all of a token type from a safety deposit box.
    /// Once fractional supply is zero and all tokens have been removed this action will take vault to Deactivated
    ///   0. `[writable]` Initialized Destination account for the tokens being withdrawn
    ///   1. `[writable]` The security deposit box account key for the tokens
    ///   2. `[writable]` The store key on the security deposit box account
    ///   3. `[writable]` The initialized combined token vault
    ///   4. `[]` Fraction mint
    ///   5. `[signer]` Authority of vault
    ///   6. `[]` PDA-based Transfer authority to move the tokens from the store to the destination seed [PREFIX, program_id]
    ///   7. `[]` Token program
    ///   8. `[]` Rent sysvar
    WithdrawTokenFromSafetyDepositBox,

    /// Self explanatory - mint more fractional shares if the vault is configured to allow such.
    ///   0. `[writable]` Fraction treasury
    ///   1. `[writable]` Fraction mint
    ///   2. `[]` The initialized active token vault
    ///   3. `[]` PDA-based Mint authority to mint tokens to treasury[PREFIX, program_id]
    ///   4. `[signer]` Authority of vault
    ///   5. `[]` Token program
    MintFractionalShares(NumberOfShareArgs),

    /// Withdraws shares from the treasury to a desired account.
    ///   0. `[writable]` Initialized Destination account for the shares being withdrawn
    ///   1. `[writable]` Fraction treasury
    ///   2. `[]` The initialized active token vault
    ///   3. `[]` PDA-based Transfer authority to move tokens from treasury to your destination[PREFIX, program_id]
    ///   3. `[signer]` Authority of vault
    ///   4. `[]` Token program
    ///   5. `[]` Rent sysvar
    WithdrawSharesFromTreasury(NumberOfShareArgs),

    /// Returns shares to the vault if you wish to remove them from circulation.
    ///   0. `[writable]` Initialized account from which shares will be withdrawn
    ///   1. `[writable]` Fraction treasury
    ///   2. `[]` The initialized active token vault
    ///   3. `[]` Transfer authority to move tokens from your account to treasury
    ///   3. `[signer]` Authority of vault
    ///   4. `[]` Token program
    AddSharesToTreasury(NumberOfShareArgs),
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
