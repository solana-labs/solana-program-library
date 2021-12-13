use crate::{get_associated_token_address, output::CliTokenAccount};
use serde::{Deserialize, Serialize};
use solana_account_decoder::{parse_token::TokenAccountType, UiAccountData};
use solana_client::rpc_response::RpcKeyedAccount;
use solana_sdk::pubkey::Pubkey;
use std::{
    collections::{btree_map::Entry, BTreeMap},
    str::FromStr,
};

pub(crate) type MintAccounts = BTreeMap<String, Vec<CliTokenAccount>>;

#[derive(Serialize, Deserialize)]
pub(crate) struct UnsupportedAccount {
    pub address: String,
    pub err: String,
}

pub(crate) fn sort_and_parse_token_accounts(
    owner: &Pubkey,
    accounts: Vec<RpcKeyedAccount>,
) -> (MintAccounts, Vec<UnsupportedAccount>, usize, bool) {
    let mut mint_accounts: MintAccounts = BTreeMap::new();
    let mut unsupported_accounts = vec![];
    let mut max_len_balance = 0;
    let mut includes_aux = false;
    for keyed_account in accounts {
        let address = keyed_account.pubkey;

        if let UiAccountData::Json(parsed_account) = keyed_account.account.data {
            if parsed_account.program != "spl-token" {
                unsupported_accounts.push(UnsupportedAccount {
                    address,
                    err: format!("Unsupported account program: {}", parsed_account.program),
                });
            } else {
                match serde_json::from_value(parsed_account.parsed) {
                    Ok(TokenAccountType::Account(ui_token_account)) => {
                        let mint = ui_token_account.mint.clone();
                        let is_associated = if let Ok(mint) = Pubkey::from_str(&mint) {
                            get_associated_token_address(owner, &mint).to_string() == address
                        } else {
                            includes_aux = true;
                            false
                        };
                        let len_balance = ui_token_account
                            .token_amount
                            .real_number_string_trimmed()
                            .len();
                        max_len_balance = max_len_balance.max(len_balance);
                        let parsed_account = CliTokenAccount {
                            address,
                            account: ui_token_account,
                            is_associated,
                        };
                        let entry = mint_accounts.entry(mint);
                        match entry {
                            Entry::Occupied(_) => {
                                entry.and_modify(|e| e.push(parsed_account));
                            }
                            Entry::Vacant(_) => {
                                entry.or_insert_with(|| vec![parsed_account]);
                            }
                        }
                    }
                    Ok(_) => unsupported_accounts.push(UnsupportedAccount {
                        address,
                        err: "Not a token account".to_string(),
                    }),
                    Err(err) => unsupported_accounts.push(UnsupportedAccount {
                        address,
                        err: format!("Account parse failure: {}", err),
                    }),
                }
            }
        } else {
            unsupported_accounts.push(UnsupportedAccount {
                address,
                err: "Unsupported account data format".to_string(),
            });
        }
    }
    for (_, array) in mint_accounts.iter_mut() {
        array.sort_by(|a, b| b.is_associated.cmp(&a.is_associated));
    }
    (
        mint_accounts,
        unsupported_accounts,
        max_len_balance,
        includes_aux,
    )
}
