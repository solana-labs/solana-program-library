use crate::native_account_data::NativeAccountData;

use spl_token::state::{Account as TokenAccount, AccountState as TokenAccountState, Mint};

use solana_program::{program_option::COption, program_pack::Pack, pubkey::Pubkey};

pub fn create_mint(owner: &Pubkey) -> NativeAccountData {
    let mut account_data = NativeAccountData::new(Mint::LEN, spl_token::id());
    let mint = Mint {
        is_initialized: true,
        mint_authority: COption::Some(*owner),
        ..Default::default()
    };
    Mint::pack(mint, &mut account_data.data[..]).unwrap();
    account_data
}

pub fn create_token_account(
    mint_account: &mut NativeAccountData,
    owner: &Pubkey,
    amount: u64,
) -> NativeAccountData {
    let mut mint = Mint::unpack(&mint_account.data).unwrap();
    let mut account_data = NativeAccountData::new(TokenAccount::LEN, spl_token::id());
    let account = TokenAccount {
        state: TokenAccountState::Initialized,
        mint: mint_account.key,
        owner: *owner,
        amount,
        ..Default::default()
    };
    mint.supply += amount;
    Mint::pack(mint, &mut mint_account.data[..]).unwrap();
    TokenAccount::pack(account, &mut account_data.data[..]).unwrap();
    account_data
}

pub fn get_token_balance(account_data: &NativeAccountData) -> u64 {
    let account = TokenAccount::unpack(&account_data.data).unwrap();
    account.amount
}

pub fn transfer(
    from_account: &mut NativeAccountData,
    to_account: &mut NativeAccountData,
    amount: u64,
) {
    let mut from = TokenAccount::unpack(&from_account.data).unwrap();
    let mut to = TokenAccount::unpack(&to_account.data).unwrap();
    assert_eq!(from.mint, to.mint);
    from.amount -= amount;
    to.amount += amount;
    TokenAccount::pack(from, &mut from_account.data[..]).unwrap();
    TokenAccount::pack(to, &mut to_account.data[..]).unwrap();
}
