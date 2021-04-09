use {
    crate::state::{ExternalPriceAccount, EXTERNAL_ACCOUNT_KEY},
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
    ///   3. `[writable]` Uninitialized vault account
    ///   4. `[]` Authority on the vault
    ///   5. `[]` Pricing Lookup Address
    ///   6. `[]` Token program
    ///   7. `[]` Rent sysvar
    InitVault(InitVaultArgs),

    /// Add a token to a inactive token vault
    ///   0. `[writable]` Uninitialized safety deposit box account address (will be created and allocated by this endpoint)
    ///                   Address should be pda with seed of [PREFIX, vault_address, token_mint_address]
    ///   1. `[writable]` Initialized Token account
    ///   2. `[writable]` Initialized Token store account with authority of this program, this will get set on the safety deposit box
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
    ///   4. `[]` Fraction mint authority for the program - seed of [PREFIX, program_id]
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
    ///   8. `[]` PDA-based Burn authority for the fraction treasury account containing the uncirculated shares seed [PREFIX, program_id]
    ///   9. `[]` External pricing lookup address
    ///   10. `[]` Token program
    CombineVault,

    /// If in the combine state, shareholders can hit this endpoint to burn shares in exchange for monies from the treasury.
    /// Once fractional supply is zero and all tokens have been removed this action will take vault to Deactivated
    ///   0. `[writable]` Initialized Token account containing your fractional shares
    ///   1. `[writable]` Initialized Destination token account where you wish your proceeds to arrive
    ///   2. `[writable]` Fraction mint
    ///   3. `[writable]` Redeem treasury account
    ///   4. `[]` PDA-based Transfer authority for the transfer of proceeds from redeem treasury to destination seed [PREFIX, program_id]
    ///   5. `[]` Burn authority for the burning of your shares
    ///   6. `[]` Combined token vault
    ///   7. `[]` Token program
    ///   8. `[]` Rent sysvar
    RedeemShares,

    /// If in combine state, authority on vault can hit this to withdrawal all of a token type from a safety deposit box.
    /// Once fractional supply is zero and all tokens have been removed this action will take vault to Deactivated
    ///   0. `[writable]` Initialized Destination account for the tokens being withdrawn
    ///   1. `[writable]` The safety deposit box account key for the tokens
    ///   2. `[writable]` The store key on the safety deposit box account
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

    /// Helpful method that isn't necessary to use for main users of the app, but allows one to create/update
    /// existing external price account fields if they are signers of this account.
    /// Useful for testing purposes, and the CLI makes use of it as well so that you can verify logic.
    ///   0. `[writable]` External price account
    UpdateExternalPriceAccount(ExternalPriceAccount),
}

/// Creates an InitVault instruction
#[allow(clippy::too_many_arguments)]
pub fn create_init_vault_instruction(
    program_id: Pubkey,
    fraction_mint: Pubkey,
    redeem_treasury: Pubkey,
    fraction_treasury: Pubkey,
    vault: Pubkey,
    vault_authority: Pubkey,
    external_price_account: Pubkey,
    allow_further_share_creation: bool,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fraction_mint, false),
            AccountMeta::new(redeem_treasury, false),
            AccountMeta::new(fraction_treasury, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(vault_authority, false),
            AccountMeta::new_readonly(external_price_account, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: VaultInstruction::InitVault(InitVaultArgs {
            allow_further_share_creation,
        })
        .try_to_vec()
        .unwrap(),
    }
}

/// Creates an UpdateExternalPriceAccount instruction
#[allow(clippy::too_many_arguments)]
pub fn create_update_external_price_account_instruction(
    program_id: Pubkey,
    external_price_account: Pubkey,
    price_per_share: u64,
    price_mint: Pubkey,
    allowed_to_combine: bool,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![AccountMeta::new(external_price_account, true)],
        data: VaultInstruction::UpdateExternalPriceAccount(ExternalPriceAccount {
            key: EXTERNAL_ACCOUNT_KEY,
            price_per_share,
            price_mint,
            allowed_to_combine,
        })
        .try_to_vec()
        .unwrap(),
    }
}

/// Creates an AddTokenToInactiveVault instruction
#[allow(clippy::too_many_arguments)]
pub fn create_add_token_to_inactive_vault_instruction(
    program_id: Pubkey,
    safety_deposit_box: Pubkey,
    token_account: Pubkey,
    store: Pubkey,
    vault: Pubkey,
    vault_authority: Pubkey,
    payer: Pubkey,
    transfer_authority: Pubkey,
    amount: u64,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(safety_deposit_box, false),
            AccountMeta::new(token_account, false),
            AccountMeta::new(store, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(vault_authority, true),
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new_readonly(transfer_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],
        data: VaultInstruction::AddTokenToInactiveVault(AddTokenToInactiveVaultArgs { amount })
            .try_to_vec()
            .unwrap(),
    }
}

/// Creates an ActivateVault instruction
#[allow(clippy::too_many_arguments)]
pub fn create_activate_vault_instruction(
    program_id: Pubkey,
    vault: Pubkey,
    fraction_mint: Pubkey,
    fraction_treasury: Pubkey,
    vault_authority: Pubkey,
    fraction_mint_authority: Pubkey,
    number_of_shares: u64,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(vault, false),
            AccountMeta::new(fraction_mint, false),
            AccountMeta::new(fraction_treasury, false),
            AccountMeta::new_readonly(vault_authority, true),
            AccountMeta::new_readonly(fraction_mint_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: VaultInstruction::ActivateVault(NumberOfShareArgs { number_of_shares })
            .try_to_vec()
            .unwrap(),
    }
}

/// Creates an CombineVault instruction
#[allow(clippy::too_many_arguments)]
pub fn create_combine_vault_instruction(
    program_id: Pubkey,
    vault: Pubkey,
    outstanding_share_token_account: Pubkey,
    paying_token_account: Pubkey,
    fraction_mint: Pubkey,
    fraction_treasury: Pubkey,
    redeem_treasury: Pubkey,
    vault_authority: Pubkey,
    paying_transfer_authority: Pubkey,
    uncirculated_burn_authority: Pubkey,
    external_pricing_account: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(vault, false),
            AccountMeta::new(outstanding_share_token_account, false),
            AccountMeta::new(paying_token_account, false),
            AccountMeta::new(fraction_mint, false),
            AccountMeta::new(fraction_treasury, false),
            AccountMeta::new(redeem_treasury, false),
            AccountMeta::new_readonly(vault_authority, true),
            AccountMeta::new_readonly(paying_transfer_authority, true),
            AccountMeta::new_readonly(uncirculated_burn_authority, true),
            AccountMeta::new_readonly(external_pricing_account, true),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: VaultInstruction::CombineVault.try_to_vec().unwrap(),
    }
}

/// Creates an RedeemShares instruction
#[allow(clippy::too_many_arguments)]
pub fn create_redeem_shares_instruction(
    program_id: Pubkey,
    outstanding_shares_account: Pubkey,
    proceeds_account: Pubkey,
    fraction_mint: Pubkey,
    redeem_treasury: Pubkey,
    transfer_authority: Pubkey,
    burn_authority: Pubkey,
    vault: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(outstanding_shares_account, false),
            AccountMeta::new(proceeds_account, false),
            AccountMeta::new(fraction_mint, false),
            AccountMeta::new(redeem_treasury, false),
            AccountMeta::new_readonly(transfer_authority, false),
            AccountMeta::new_readonly(burn_authority, false),
            AccountMeta::new_readonly(vault, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: VaultInstruction::RedeemShares.try_to_vec().unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_withdraw_tokens_instruction(
    program_id: Pubkey,
    destination: Pubkey,
    safety_deposit_box: Pubkey,
    store: Pubkey,
    vault: Pubkey,
    fraction_mint: Pubkey,
    vault_authority: Pubkey,
    transfer_authority: Pubkey,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(destination, false),
            AccountMeta::new(safety_deposit_box, false),
            AccountMeta::new(store, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(fraction_mint, false),
            AccountMeta::new_readonly(vault_authority, true),
            AccountMeta::new_readonly(transfer_authority, false),
            AccountMeta::new_readonly(spl_token::id(), false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        data: VaultInstruction::WithdrawTokenFromSafetyDepositBox
            .try_to_vec()
            .unwrap(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn create_mint_shares_instruction(
    program_id: Pubkey,
    fraction_treasury: Pubkey,
    fraction_mint: Pubkey,
    vault: Pubkey,
    fraction_mint_authority: Pubkey,
    vault_authority: Pubkey,
    number_of_shares: u64,
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(fraction_treasury, false),
            AccountMeta::new(fraction_mint, false),
            AccountMeta::new_readonly(vault, false),
            AccountMeta::new_readonly(fraction_mint_authority, false),
            AccountMeta::new_readonly(vault_authority, true),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: VaultInstruction::MintFractionalShares(NumberOfShareArgs { number_of_shares })
            .try_to_vec()
            .unwrap(),
    }
}
