use crate::{
    output::{CliTokenAccount, CliTokenAccounts},
    Error,
};
use serde::{Deserialize, Serialize};
use solana_account_decoder::{parse_token::TokenAccountType, UiAccountData};
use solana_client::rpc_response::RpcKeyedAccount;
use solana_sdk::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use std::{
    collections::{btree_map::Entry, BTreeMap},
    str::FromStr,
};

#[derive(Serialize, Deserialize)]
pub(crate) struct UnsupportedAccount {
    pub address: String,
    pub err: String,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub(crate) enum AccountFilter {
    Delegated,
    ExternallyCloseable,
    All,
}

pub(crate) fn sort_and_parse_token_accounts(
    owner: &Pubkey,
    accounts: Vec<RpcKeyedAccount>,
    explicit_token: bool,
    account_filter: AccountFilter,
) -> Result<CliTokenAccounts, Error> {
    let mut cli_accounts: BTreeMap<(Pubkey, Pubkey), Vec<CliTokenAccount>> = BTreeMap::new();
    let mut unsupported_accounts = vec![];
    let mut max_len_balance = 0;
    let mut aux_count = 0;

    for keyed_account in accounts {
        let address_str = keyed_account.pubkey;
        let address = Pubkey::from_str(&address_str)?;
        let program_id = Pubkey::from_str(&keyed_account.account.owner)?;

        if let UiAccountData::Json(parsed_account) = keyed_account.account.data {
            match serde_json::from_value(parsed_account.parsed) {
                Ok(TokenAccountType::Account(ui_token_account)) => {
                    let mint = Pubkey::from_str(&ui_token_account.mint)?;
                    let btree_key = (program_id, mint);
                    let is_associated =
                        get_associated_token_address_with_program_id(owner, &mint, &program_id)
                            == address;

                    match account_filter {
                        AccountFilter::Delegated if ui_token_account.delegate.is_none() => continue,
                        AccountFilter::ExternallyCloseable
                            if ui_token_account.close_authority.is_none() =>
                        {
                            continue
                        }
                        _ => (),
                    }

                    if !is_associated {
                        aux_count += 1;
                    }

                    max_len_balance = max_len_balance.max(
                        ui_token_account
                            .token_amount
                            .real_number_string_trimmed()
                            .len(),
                    );

                    let cli_account = CliTokenAccount {
                        address: address_str,
                        program_id: program_id.to_string(),
                        account: ui_token_account,
                        is_associated,
                    };

                    let entry = cli_accounts.entry(btree_key);
                    match entry {
                        Entry::Occupied(_) => {
                            entry.and_modify(|e| {
                                if is_associated {
                                    e.insert(0, cli_account)
                                } else {
                                    e.push(cli_account)
                                }
                            });
                        }
                        Entry::Vacant(_) => {
                            entry.or_insert_with(|| vec![cli_account]);
                        }
                    }
                }
                Ok(_) => unsupported_accounts.push(UnsupportedAccount {
                    address: address_str,
                    err: "Not a token account".to_string(),
                }),
                Err(err) => unsupported_accounts.push(UnsupportedAccount {
                    address: address_str,
                    err: format!("Account parse failure: {}", err),
                }),
            }
        }
    }

    Ok(CliTokenAccounts {
        accounts: cli_accounts
            .into_iter()
            .map(|(_key, accounts_list)| accounts_list)
            .collect(),
        unsupported_accounts,
        max_len_balance,
        aux_len: if aux_count > 0 {
            format!("  (Aux-{}*)", aux_count).chars().count() + 1
        } else {
            0
        },
        explicit_token,
    })
}
