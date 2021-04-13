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
pub struct InitMetaplexArgs {
    pub allow_further_share_creation: bool,
}

/// Instructions supported by the Fraction program.
#[derive(BorshSerialize, BorshDeserialize, Clone)]
pub enum MetaplexInstruction {
    /// Initialize a token vault, starts inactivate. Add tokens in subsequent instructions, then activate.
    ///   0. `[writable]` Initialized fractional share mint with 0 tokens in supply, authority on mint must be pda of program with seed [prefix, programid]
    ///   1. `[writable]` Initialized redeem treasury token account with 0 tokens in supply, owner of account must be pda of program like above
    ///   2. `[writable]` Initialized fraction treasury token account with 0 tokens in supply, owner of account must be pda of program like above
    ///   3. `[writable]` Uninitialized vault account
    ///   4. `[]` Authority on the vault
    ///   5. `[]` Pricing Lookup Address
    ///   6. `[]` Token program
    ///   7. `[]` Rent sysvar
    InitMetaplex(InitMetaplexArgs),
}
/*
/// Creates an InitMetaplex instruction
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
        data: MetaplexInstruction::InitMetaplex(InitMetaplexArgs {
            allow_further_share_creation,
        })
        .try_to_vec()
        .unwrap(),
    }
}
*/
