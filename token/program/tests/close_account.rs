#![cfg(feature = "test-sbf")]

mod setup;

use {
    mollusk_svm::{result::Check, Mollusk},
    solana_sdk::{
        account::{AccountSharedData, ReadableAccount},
        program_error::ProgramError,
        program_pack::Pack,
        pubkey::Pubkey,
        system_instruction, system_program,
    },
    spl_token::{instruction, state::Account},
};

#[test]
fn success_init_after_close_account() {
    let mollusk = Mollusk::new(&spl_token::id(), "spl_token");

    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let account = Pubkey::new_unique();
    let decimals = 9;

    let owner_account = AccountSharedData::new(1_000_000_000, 0, &system_program::id());
    let mint_account = setup::setup_mint_account(None, None, 0, decimals);
    let token_account = setup::setup_token_account(&mint, &owner, 0);

    mollusk.process_and_validate_instruction_chain(
        &[
            instruction::close_account(&spl_token::id(), &account, &owner, &owner, &[]).unwrap(),
            system_instruction::create_account(
                &owner,
                &account,
                1_000_000_000,
                Account::LEN as u64,
                &spl_token::id(),
            ),
            instruction::initialize_account(&spl_token::id(), &account, &mint, &owner).unwrap(),
        ],
        &[
            (mint, mint_account),
            (account, token_account),
            (owner, owner_account),
            mollusk.sysvars.keyed_account_for_rent_sysvar(),
        ],
        &[
            Check::success(),
            // Account successfully initialized.
            Check::account(&account)
                .data(setup::setup_token_account(&mint, &owner, 0).data())
                .owner(&spl_token::id())
                .build(),
        ],
    );
}

#[test]
fn fail_init_after_close_account() {
    let mollusk = Mollusk::new(&spl_token::id(), "spl_token");

    let owner = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let account = Pubkey::new_unique();
    let decimals = 9;

    let owner_account = AccountSharedData::new(1_000_000_000, 0, &system_program::id());
    let mint_account = setup::setup_mint_account(None, None, 0, decimals);
    let token_account = setup::setup_token_account(&mint, &owner, 0);

    mollusk.process_and_validate_instruction_chain(
        &[
            instruction::close_account(&spl_token::id(), &account, &owner, &owner, &[]).unwrap(),
            system_instruction::transfer(&owner, &account, 1_000_000_000),
            instruction::initialize_account(&spl_token::id(), &account, &mint, &owner).unwrap(),
        ],
        &[
            (mint, mint_account),
            (account, token_account),
            (owner, owner_account),
            mollusk.sysvars.keyed_account_for_rent_sysvar(),
        ],
        &[
            Check::err(ProgramError::InvalidAccountData),
            // Account not initialized.
            Check::account(&account)
                .lamports(1_000_000_000)
                .owner(&system_program::id())
                .build(),
        ],
    );
}
