#![cfg(feature = "test-sbf")]

use {
    solana_sdk::{
        account::{Account as SolanaAccount, AccountSharedData},
        program_pack::Pack,
        pubkey::Pubkey,
        rent::Rent,
    },
    spl_token::state::{Account, AccountState, Mint},
};

pub fn setup_mint_account(
    mint_authority: Option<&Pubkey>,
    freeze_authority: Option<&Pubkey>,
    supply: u64,
    decimals: u8,
) -> AccountSharedData {
    let data = {
        let mut data = vec![0; Mint::LEN];
        let state = Mint {
            mint_authority: mint_authority.cloned().into(),
            supply,
            decimals,
            is_initialized: true,
            freeze_authority: freeze_authority.cloned().into(),
        };
        state.pack_into_slice(&mut data);
        data
    };

    let space = data.len();
    let lamports = Rent::default().minimum_balance(space);

    AccountSharedData::from(SolanaAccount {
        lamports,
        data,
        owner: spl_token::id(),
        ..Default::default()
    })
}

pub fn setup_token_account(mint: &Pubkey, owner: &Pubkey, amount: u64) -> AccountSharedData {
    let data = {
        let mut data = vec![0; Account::LEN];
        let state = Account {
            mint: *mint,
            owner: *owner,
            amount,
            delegate: None.into(),
            state: AccountState::Initialized,
            is_native: None.into(),
            delegated_amount: 0,
            close_authority: None.into(),
        };
        state.pack_into_slice(&mut data);
        data
    };

    let space = data.len();
    let lamports = Rent::default().minimum_balance(space);

    AccountSharedData::from(SolanaAccount {
        lamports,
        data,
        owner: spl_token::id(),
        ..Default::default()
    })
}
