use clap::{
    crate_description, crate_name, crate_version, value_t_or_exit, App, AppSettings, Arg,
    ArgMatches, SubCommand,
};
use console::Emoji;
use solana_account_decoder::{
    parse_token::{TokenAccountType, UiAccountState, UiTokenAmount},
    UiAccountData,
};
use solana_clap_utils::{
    fee_payer::fee_payer_arg,
    input_parsers::{pubkey_of_signer, pubkeys_of_multiple_signers, signer_of, value_of},
    input_validators::{
        is_amount, is_amount_or_all, is_parsable, is_url, is_valid_pubkey, is_valid_signer,
    },
    keypair::{pubkey_from_path, signer_from_path, DefaultSigner},
    nonce::*,
    offline::{self, *},
    ArgConstant,
};
use solana_cli_output::{display::println_name_value, return_signers, OutputFormat};
use solana_client::{
    blockhash_query::BlockhashQuery, rpc_client::RpcClient, rpc_request::TokenAccountsFilter,
};
use solana_remote_wallet::remote_wallet::RemoteWalletManager;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    message::Message,
    native_token::*,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_instruction, system_program,
    transaction::Transaction,
};
use spl_associated_token_account::*;
use spl_token::{
    self,
    instruction::*,
    native_mint,
    state::{Account, Mint, Multisig},
};
use std::{collections::HashMap, process::exit, str::FromStr, sync::Arc};

static WARNING: Emoji = Emoji("⚠️", "!");

pub const MINT_ADDRESS_ARG: ArgConstant<'static> = ArgConstant {
    name: "mint_address",
    long: "mint-address",
    help: "Address of mint that token account is associated with. Required by --sign-only",
};

pub const MINT_DECIMALS_ARG: ArgConstant<'static> = ArgConstant {
    name: "mint_decimals",
    long: "mint-decimals",
    help: "Decimals of mint that token account is associated with. Required by --sign-only",
};

pub const DELEGATE_ADDRESS_ARG: ArgConstant<'static> = ArgConstant {
    name: "delegate_address",
    long: "delegate-address",
    help: "Address of delegate currently assigned to token account. Required by --sign-only",
};

pub const MULTISIG_SIGNER_ARG: ArgConstant<'static> = ArgConstant {
    name: "multisig_signer",
    long: "multisig-signer",
    help: "Member signer of a multisig account",
};

pub fn mint_address_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(MINT_ADDRESS_ARG.name)
        .long(MINT_ADDRESS_ARG.long)
        .takes_value(true)
        .value_name("MINT_ADDRESS")
        .validator(is_valid_pubkey)
        .requires(SIGN_ONLY_ARG.name)
        .requires(BLOCKHASH_ARG.name)
        .help(MINT_ADDRESS_ARG.help)
}

fn is_mint_decimals(string: String) -> Result<(), String> {
    is_parsable::<u8>(string)
}

pub fn mint_decimals_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(MINT_DECIMALS_ARG.name)
        .long(MINT_DECIMALS_ARG.long)
        .takes_value(true)
        .value_name("MINT_DECIMALS")
        .validator(is_mint_decimals)
        .requires(SIGN_ONLY_ARG.name)
        .requires(BLOCKHASH_ARG.name)
        .help(MINT_DECIMALS_ARG.help)
}

pub trait MintArgs {
    fn mint_args(self) -> Self;
}

impl MintArgs for App<'_, '_> {
    fn mint_args(self) -> Self {
        self.arg(mint_address_arg().requires(MINT_DECIMALS_ARG.name))
            .arg(mint_decimals_arg().requires(MINT_ADDRESS_ARG.name))
    }
}

pub fn delegate_address_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(DELEGATE_ADDRESS_ARG.name)
        .long(DELEGATE_ADDRESS_ARG.long)
        .takes_value(true)
        .value_name("DELEGATE_ADDRESS")
        .validator(is_valid_pubkey)
        .requires(SIGN_ONLY_ARG.name)
        .requires(BLOCKHASH_ARG.name)
        .help(DELEGATE_ADDRESS_ARG.help)
}

pub fn multisig_signer_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(MULTISIG_SIGNER_ARG.name)
        .long(MULTISIG_SIGNER_ARG.long)
        .validator(is_valid_signer)
        .value_name("MULTISIG_SIGNER")
        .takes_value(true)
        .multiple(true)
        .min_values(0u64)
        .max_values(MAX_SIGNERS as u64)
        .help(MULTISIG_SIGNER_ARG.help)
}

fn is_multisig_minimum_signers(string: String) -> Result<(), String> {
    let v = u8::from_str(&string).map_err(|e| e.to_string())? as usize;
    if v < MIN_SIGNERS {
        Err(format!("must be at least {}", MIN_SIGNERS))
    } else if v > MAX_SIGNERS {
        Err(format!("must be at most {}", MAX_SIGNERS))
    } else {
        Ok(())
    }
}

struct Config<'a> {
    rpc_client: RpcClient,
    verbose: bool,
    owner: Pubkey,
    fee_payer: Pubkey,
    default_signer: DefaultSigner,
    nonce_account: Option<Pubkey>,
    nonce_authority: Option<Pubkey>,
    blockhash_query: BlockhashQuery,
    sign_only: bool,
    multisigner_pubkeys: Vec<&'a Pubkey>,
}

type Error = Box<dyn std::error::Error>;
type CommandResult = Result<Option<(u64, Vec<Vec<Instruction>>)>, Error>;

fn new_throwaway_signer() -> (Option<Box<dyn Signer>>, Option<Pubkey>) {
    let keypair = Keypair::new();
    let pubkey = keypair.pubkey();
    (Some(Box::new(keypair) as Box<dyn Signer>), Some(pubkey))
}

fn check_fee_payer_balance(config: &Config, required_balance: u64) -> Result<(), Error> {
    let balance = config.rpc_client.get_balance(&config.fee_payer)?;
    if balance < required_balance {
        Err(format!(
            "Fee payer, {}, has insufficient balance: {} required, {} available",
            config.fee_payer,
            lamports_to_sol(required_balance),
            lamports_to_sol(balance)
        )
        .into())
    } else {
        Ok(())
    }
}

fn check_owner_balance(config: &Config, required_balance: u64) -> Result<(), Error> {
    let balance = config.rpc_client.get_balance(&config.owner)?;
    if balance < required_balance {
        Err(format!(
            "Owner, {}, has insufficient balance: {} required, {} available",
            config.owner,
            lamports_to_sol(required_balance),
            lamports_to_sol(balance)
        )
        .into())
    } else {
        Ok(())
    }
}

type SignersOf = Vec<(Box<dyn Signer>, Pubkey)>;
pub fn signers_of(
    matches: &ArgMatches<'_>,
    name: &str,
    wallet_manager: &mut Option<Arc<RemoteWalletManager>>,
) -> Result<Option<SignersOf>, Box<dyn std::error::Error>> {
    if let Some(values) = matches.values_of(name) {
        let mut results = Vec::new();
        for (i, value) in values.enumerate() {
            let name = format!("{}-{}", name, i + 1);
            let signer = signer_from_path(matches, value, &name, wallet_manager)?;
            let signer_pubkey = signer.pubkey();
            results.push((signer, signer_pubkey));
        }
        Ok(Some(results))
    } else {
        Ok(None)
    }
}

fn command_create_token(
    config: &Config,
    decimals: u8,
    token: Pubkey,
    enable_freeze: bool,
) -> CommandResult {
    println!("Creating token {}", token);

    let minimum_balance_for_rent_exemption = if !config.sign_only {
        config
            .rpc_client
            .get_minimum_balance_for_rent_exemption(Mint::LEN)?
    } else {
        0
    };
    let freeze_authority_pubkey = if enable_freeze {
        Some(config.owner)
    } else {
        None
    };

    let instructions = vec![
        system_instruction::create_account(
            &config.fee_payer,
            &token,
            minimum_balance_for_rent_exemption,
            Mint::LEN as u64,
            &spl_token::id(),
        ),
        initialize_mint(
            &spl_token::id(),
            &token,
            &config.owner,
            freeze_authority_pubkey.as_ref(),
            decimals,
        )?,
    ];
    Ok(Some((
        minimum_balance_for_rent_exemption,
        vec![instructions],
    )))
}

fn command_create_account(
    config: &Config,
    token: Pubkey,
    maybe_account: Option<Pubkey>,
) -> CommandResult {
    let minimum_balance_for_rent_exemption = if !config.sign_only {
        config
            .rpc_client
            .get_minimum_balance_for_rent_exemption(Account::LEN)?
    } else {
        0
    };

    let (account, system_account_ok, instructions) = if let Some(account) = maybe_account {
        println!("Creating account {}", account);
        (
            account,
            false,
            vec![
                system_instruction::create_account(
                    &config.fee_payer,
                    &account,
                    minimum_balance_for_rent_exemption,
                    Account::LEN as u64,
                    &spl_token::id(),
                ),
                initialize_account(&spl_token::id(), &account, &token, &config.owner)?,
            ],
        )
    } else {
        let account = get_associated_token_address(&config.owner, &token);
        println!("Creating account {}", account);
        (
            account,
            true,
            vec![create_associated_token_account(
                &config.fee_payer,
                &config.owner,
                &token,
            )],
        )
    };

    if let Some(account_data) = config
        .rpc_client
        .get_account_with_commitment(&account, config.rpc_client.commitment())?
        .value
    {
        if !(account_data.owner == system_program::id() && system_account_ok) {
            return Err(format!("Error: Account already exists: {}", account).into());
        }
    }

    Ok(Some((
        minimum_balance_for_rent_exemption,
        vec![instructions],
    )))
}

fn command_create_multisig(
    config: &Config,
    multisig: Pubkey,
    minimum_signers: u8,
    multisig_members: Vec<Pubkey>,
) -> CommandResult {
    println!(
        "Creating {}/{} multisig {}",
        minimum_signers,
        multisig_members.len(),
        multisig
    );

    let minimum_balance_for_rent_exemption = if !config.sign_only {
        config
            .rpc_client
            .get_minimum_balance_for_rent_exemption(Multisig::LEN)?
    } else {
        0
    };

    let instructions = vec![
        system_instruction::create_account(
            &config.fee_payer,
            &multisig,
            minimum_balance_for_rent_exemption,
            Multisig::LEN as u64,
            &spl_token::id(),
        ),
        initialize_multisig(
            &spl_token::id(),
            &multisig,
            multisig_members.iter().collect::<Vec<_>>().as_slice(),
            minimum_signers,
        )?,
    ];
    Ok(Some((
        minimum_balance_for_rent_exemption,
        vec![instructions],
    )))
}

fn command_authorize(
    config: &Config,
    account: Pubkey,
    authority_type: AuthorityType,
    new_owner: Option<Pubkey>,
) -> CommandResult {
    let auth_str = match authority_type {
        AuthorityType::MintTokens => "mint authority",
        AuthorityType::FreezeAccount => "freeze authority",
        AuthorityType::AccountOwner => "owner",
        AuthorityType::CloseAccount => "close authority",
    };
    println!(
        "Updating {}\n  Current {}: {}\n  New {}: {}",
        account,
        auth_str,
        config.owner,
        auth_str,
        new_owner
            .map(|pubkey| pubkey.to_string())
            .unwrap_or_else(|| "disabled".to_string())
    );

    let instructions = vec![set_authority(
        &spl_token::id(),
        &account,
        new_owner.as_ref(),
        authority_type,
        &config.owner,
        &config.multisigner_pubkeys,
    )?];
    Ok(Some((0, vec![instructions])))
}

fn resolve_mint_info(
    config: &Config,
    token_account: &Pubkey,
    mint_address: Option<Pubkey>,
    mint_decimals: Option<u8>,
) -> Result<(Pubkey, u8), Error> {
    if !config.sign_only {
        let source_account = config
            .rpc_client
            .get_token_account(&token_account)?
            .ok_or_else(|| format!("Could not find token account {}", token_account))?;
        Ok((
            Pubkey::from_str(&source_account.mint)?,
            source_account.token_amount.decimals,
        ))
    } else {
        Ok((
            mint_address.unwrap_or_default(),
            mint_decimals.unwrap_or_default(),
        ))
    }
}

fn command_transfer(
    config: &Config,
    sender: Pubkey,
    ui_amount: Option<f64>,
    recipient: Pubkey,
    fund_recipient: bool,
    mint_address: Option<Pubkey>,
    mint_decimals: Option<u8>,
) -> CommandResult {
    let sender_balance = config
        .rpc_client
        .get_token_account_balance(&sender)
        .map_err(|err| {
            format!(
                "Error: Failed to get token balance of sender address {}: {}",
                sender, err
            )
        })?
        .ui_amount;
    let ui_amount = ui_amount.unwrap_or(sender_balance);

    println!(
        "Transfer {} tokens\n  Sender: {}\n  Recipient: {}",
        ui_amount, sender, recipient
    );
    if ui_amount > sender_balance {
        return Err(format!(
            "Error: Sender has insufficient funds, current balance is {}",
            sender_balance
        )
        .into());
    }

    let (mint_pubkey, decimals) = resolve_mint_info(config, &sender, mint_address, mint_decimals)?;
    let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);
    let mut instructions = vec![];

    let mut recipient_token_account = recipient;
    let mut minimum_balance_for_rent_exemption = 0;

    if let Some(account_data) = config
        .rpc_client
        .get_account_with_commitment(&recipient, config.rpc_client.commitment())?
        .value
    {
        if account_data.owner == system_program::id() {
            recipient_token_account = get_associated_token_address(&recipient, &mint_pubkey);
            println!(
                "  Recipient associated token account: {}",
                recipient_token_account
            );

            let needs_funding = if let Some(recipient_token_account_data) = config
                .rpc_client
                .get_account_with_commitment(
                    &recipient_token_account,
                    config.rpc_client.commitment(),
                )?
                .value
            {
                if recipient_token_account_data.owner == system_program::id() {
                    true
                } else if recipient_token_account_data.owner == spl_token::id() {
                    false
                } else {
                    return Err(
                        format!("Error: Unsupported recipient address: {}", recipient).into(),
                    );
                }
            } else {
                true
            };

            if needs_funding {
                if fund_recipient {
                    minimum_balance_for_rent_exemption += config
                        .rpc_client
                        .get_minimum_balance_for_rent_exemption(Account::LEN)?;
                    println!(
                        "  Funding recipient: {} ({} SOL)",
                        recipient_token_account,
                        lamports_to_sol(minimum_balance_for_rent_exemption)
                    );
                    instructions.push(create_associated_token_account(
                        &config.fee_payer,
                        &recipient,
                        &mint_pubkey,
                    ));
                } else {
                    return Err(
                        "Error: Recipient's associated token account does not exist. \
                                        Add `--fund-recipient` to fund their account"
                            .into(),
                    );
                }
            }
        } else if account_data.owner != spl_token::id() {
            return Err(format!("Error: Unsupported recipient address: {}", recipient).into());
        }
    } else {
        return Err(format!("Error: Recipient does not exist: {}", recipient).into());
    }

    instructions.push(transfer_checked(
        &spl_token::id(),
        &sender,
        &mint_pubkey,
        &recipient_token_account,
        &config.owner,
        &config.multisigner_pubkeys,
        amount,
        decimals,
    )?);
    Ok(Some((
        minimum_balance_for_rent_exemption,
        vec![instructions],
    )))
}

fn command_burn(
    config: &Config,
    source: Pubkey,
    ui_amount: f64,
    mint_address: Option<Pubkey>,
    mint_decimals: Option<u8>,
) -> CommandResult {
    println!("Burn {} tokens\n  Source: {}", ui_amount, source);

    let (mint_pubkey, decimals) = resolve_mint_info(config, &source, mint_address, mint_decimals)?;
    let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);

    let instructions = vec![burn_checked(
        &spl_token::id(),
        &source,
        &mint_pubkey,
        &config.owner,
        &config.multisigner_pubkeys,
        amount,
        decimals,
    )?];
    Ok(Some((0, vec![instructions])))
}

fn command_mint(
    config: &Config,
    token: Pubkey,
    ui_amount: f64,
    recipient: Pubkey,
    mint_decimals: Option<u8>,
) -> CommandResult {
    println!(
        "Minting {} tokens\n  Token: {}\n  Recipient: {}",
        ui_amount, token, recipient
    );

    let (_, decimals) = resolve_mint_info(config, &recipient, None, mint_decimals)?;
    let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);

    let instructions = vec![mint_to_checked(
        &spl_token::id(),
        &token,
        &recipient,
        &config.owner,
        &config.multisigner_pubkeys,
        amount,
        decimals,
    )?];
    Ok(Some((0, vec![instructions])))
}

fn command_freeze(config: &Config, account: Pubkey, mint_address: Option<Pubkey>) -> CommandResult {
    let (token, _) = resolve_mint_info(config, &account, mint_address, None)?;

    println!("Freezing account: {}\n  Token: {}", account, token);

    let instructions = vec![freeze_account(
        &spl_token::id(),
        &account,
        &token,
        &config.owner,
        &config.multisigner_pubkeys,
    )?];
    Ok(Some((0, vec![instructions])))
}

fn command_thaw(config: &Config, account: Pubkey, mint_address: Option<Pubkey>) -> CommandResult {
    let (token, _) = resolve_mint_info(config, &account, mint_address, None)?;

    println!("Freezing account: {}\n  Token: {}", account, token);

    let instructions = vec![thaw_account(
        &spl_token::id(),
        &account,
        &token,
        &config.owner,
        &config.multisigner_pubkeys,
    )?];
    Ok(Some((0, vec![instructions])))
}

fn command_wrap(config: &Config, sol: f64, account: Pubkey) -> CommandResult {
    let lamports = sol_to_lamports(sol);
    println!("Wrapping {} SOL into {}", sol, account);

    let instructions = vec![
        system_instruction::create_account(
            &config.owner,
            &account,
            lamports,
            Account::LEN as u64,
            &spl_token::id(),
        ),
        initialize_account(
            &spl_token::id(),
            &account,
            &native_mint::id(),
            &config.owner,
        )?,
    ];
    if !config.sign_only {
        check_owner_balance(config, lamports)?;
    }
    Ok(Some((0, vec![instructions])))
}

fn command_unwrap(config: &Config, address: Pubkey) -> CommandResult {
    println!("Unwrapping {}", address);
    if !config.sign_only {
        println!(
            "  Amount: {} SOL",
            lamports_to_sol(config.rpc_client.get_balance(&address)?),
        );
    }
    println!("  Recipient: {}", &config.owner);

    let instructions = vec![close_account(
        &spl_token::id(),
        &address,
        &config.owner,
        &config.owner,
        &config.multisigner_pubkeys,
    )?];
    Ok(Some((0, vec![instructions])))
}

fn command_approve(
    config: &Config,
    account: Pubkey,
    ui_amount: f64,
    delegate: Pubkey,
    mint_address: Option<Pubkey>,
    mint_decimals: Option<u8>,
) -> CommandResult {
    println!(
        "Approve {} tokens\n  Account: {}\n  Delegate: {}",
        ui_amount, account, delegate
    );

    let (mint_pubkey, decimals) = resolve_mint_info(config, &account, mint_address, mint_decimals)?;
    let amount = spl_token::ui_amount_to_amount(ui_amount, decimals);

    let instructions = vec![approve_checked(
        &spl_token::id(),
        &account,
        &mint_pubkey,
        &delegate,
        &config.owner,
        &config.multisigner_pubkeys,
        amount,
        decimals,
    )?];
    Ok(Some((0, vec![instructions])))
}

fn command_revoke(config: &Config, account: Pubkey, delegate: Option<Pubkey>) -> CommandResult {
    let delegate = if !config.sign_only {
        let source_account = config
            .rpc_client
            .get_token_account(&account)?
            .ok_or_else(|| format!("Could not find token account {}", account))?;
        if let Some(string) = source_account.delegate {
            Some(Pubkey::from_str(&string)?)
        } else {
            None
        }
    } else {
        delegate
    };

    if let Some(delegate) = delegate {
        println!(
            "Revoking approval\n  Account: {}\n  Delegate: {}",
            account, delegate
        );
    } else {
        return Err(format!("No delegate on account {}", account).into());
    }

    let instructions = vec![revoke(
        &spl_token::id(),
        &account,
        &config.owner,
        &config.multisigner_pubkeys,
    )?];
    Ok(Some((0, vec![instructions])))
}

fn command_close(config: &Config, account: Pubkey, destination: Pubkey) -> CommandResult {
    if !config.sign_only {
        let source_account = config
            .rpc_client
            .get_token_account(&account)?
            .ok_or_else(|| format!("Could not find token account {}", account))?;

        if !source_account.is_native && source_account.token_amount.ui_amount > 0.0 {
            return Err(format!(
                "Account {} still has {} tokens; empty the account in order to close it.",
                account, source_account.token_amount.ui_amount
            )
            .into());
        }
    }

    let instructions = vec![close_account(
        &spl_token::id(),
        &account,
        &destination,
        &config.owner,
        &config.multisigner_pubkeys,
    )?];
    Ok(Some((0, vec![instructions])))
}

fn command_balance(config: &Config, address: Pubkey) -> CommandResult {
    let balance = config.rpc_client.get_token_account_balance(&address)?;

    if config.verbose {
        println!("ui amount: {}", balance.ui_amount);
        println!("decimals: {}", balance.decimals);
        println!("amount: {}", balance.amount);
    } else {
        println!("{}", balance.ui_amount);
    }
    Ok(None)
}

fn command_supply(config: &Config, address: Pubkey) -> CommandResult {
    let supply = config.rpc_client.get_token_supply(&address)?;

    println!("{}", supply.ui_amount);
    Ok(None)
}

fn command_accounts(config: &Config, token: Option<Pubkey>) -> CommandResult {
    let accounts = config.rpc_client.get_token_accounts_by_owner(
        &config.owner,
        match token {
            Some(token) => TokenAccountsFilter::Mint(token),
            None => TokenAccountsFilter::ProgramId(spl_token::id()),
        },
    )?;
    if accounts.is_empty() {
        println!("None");
    }

    if token.is_some() {
        println!("Account                                      Balance");
        println!("----------------------------------------------------");
    } else {
        println!("Account                                      Token                                        Balance");
        println!("-------------------------------------------------------------------------------------------------");
    }
    for keyed_account in accounts {
        let address = keyed_account.pubkey;

        if let UiAccountData::Json(parsed_account) = keyed_account.account.data {
            if parsed_account.program != "spl-token" {
                println!(
                    "{:<44} Unsupported account program: {}",
                    address, parsed_account.program
                );
            } else {
                match serde_json::from_value(parsed_account.parsed) {
                    Ok(TokenAccountType::Account(ui_token_account)) => {
                        let maybe_frozen = if let UiAccountState::Frozen = ui_token_account.state {
                            format!(" {}  Frozen", WARNING)
                        } else {
                            "".to_string()
                        };
                        if token.is_some() {
                            println!(
                                "{:<44} {}{}",
                                address, ui_token_account.token_amount.ui_amount, maybe_frozen
                            )
                        } else {
                            println!(
                                "{:<44} {:<44} {}{}",
                                address,
                                ui_token_account.mint,
                                ui_token_account.token_amount.ui_amount,
                                maybe_frozen
                            )
                        }
                    }
                    Ok(_) => println!("{:<44} Unsupported token account", address),
                    Err(err) => println!("{:<44} Account parse failure: {}", address, err),
                }
            }
        } else {
            println!("{:<44} Unsupported account data format", address);
        }
    }
    Ok(None)
}

fn command_gc(config: &Config) -> CommandResult {
    println!("Fetching token accounts");
    let accounts = config.rpc_client.get_token_accounts_by_owner(
        &config.owner,
        TokenAccountsFilter::ProgramId(spl_token::id()),
    )?;
    if accounts.is_empty() {
        println!("Nothing to do");
        return Ok(None);
    }

    let minimum_balance_for_rent_exemption = if !config.sign_only {
        config
            .rpc_client
            .get_minimum_balance_for_rent_exemption(Account::LEN)?
    } else {
        0
    };

    let mut accounts_by_token = HashMap::new();

    for keyed_account in accounts {
        if let UiAccountData::Json(parsed_account) = keyed_account.account.data {
            if parsed_account.program == "spl-token" {
                if let Ok(TokenAccountType::Account(ui_token_account)) =
                    serde_json::from_value(parsed_account.parsed)
                {
                    let frozen = ui_token_account.state == UiAccountState::Frozen;

                    let token = ui_token_account
                        .mint
                        .parse::<Pubkey>()
                        .unwrap_or_else(|err| panic!("Invalid mint: {}", err));
                    let token_account = keyed_account
                        .pubkey
                        .parse::<Pubkey>()
                        .unwrap_or_else(|err| panic!("Invalid token account: {}", err));

                    let close_authority =
                        ui_token_account.close_authority.map_or(config.owner, |s| {
                            s.parse::<Pubkey>()
                                .unwrap_or_else(|err| panic!("Invalid close authority: {}", err))
                        });

                    let entry = accounts_by_token.entry(token).or_insert_with(HashMap::new);
                    entry.insert(
                        token_account,
                        (
                            spl_token::ui_amount_to_amount(
                                ui_token_account.token_amount.ui_amount,
                                ui_token_account.token_amount.decimals,
                            ),
                            ui_token_account.token_amount.decimals,
                            frozen,
                            close_authority,
                        ),
                    );
                }
            }
        }
    }

    let mut instructions = vec![];
    let mut lamports_needed = 0;

    for (token, accounts) in accounts_by_token.into_iter() {
        println!("Processing token: {}", token);
        let associated_token_account = get_associated_token_address(&config.owner, &token);
        let total_balance: u64 = accounts.values().map(|account| account.0).sum();

        if total_balance > 0 && !accounts.contains_key(&associated_token_account) {
            // Create the associated token account
            instructions.push(vec![create_associated_token_account(
                &config.fee_payer,
                &config.owner,
                &token,
            )]);
            lamports_needed += minimum_balance_for_rent_exemption;
        }

        for (address, (amount, decimals, frozen, close_authority)) in accounts {
            if address == associated_token_account {
                // leave the associated token account alone
                continue;
            }

            if frozen {
                // leave frozen accounts alone
                continue;
            }

            let mut account_instructions = vec![];

            // Transfer the account balance into the associated token account
            if amount > 0 {
                account_instructions.push(transfer_checked(
                    &spl_token::id(),
                    &address,
                    &token,
                    &associated_token_account,
                    &config.owner,
                    &config.multisigner_pubkeys,
                    amount,
                    decimals,
                )?);
            }
            // Close the account if config.owner is able to
            if close_authority == config.owner {
                account_instructions.push(close_account(
                    &spl_token::id(),
                    &address,
                    &config.owner,
                    &config.owner,
                    &config.multisigner_pubkeys,
                )?);
            }

            if !account_instructions.is_empty() {
                instructions.push(account_instructions);
            }
        }
    }

    Ok(Some((lamports_needed, instructions)))
}

fn stringify_ui_token_amount(amount: &UiTokenAmount) -> String {
    let decimals = amount.decimals as usize;
    if decimals > 0 {
        let amount = u64::from_str(&amount.amount).unwrap();

        // Left-pad zeros to decimals + 1, so we at least have an integer zero
        let mut s = format!("{:01$}", amount, decimals + 1);

        // Add the decimal point (Sorry, "," locales!)
        s.insert(s.len() - decimals, '.');
        s
    } else {
        amount.amount.clone()
    }
}

fn stringify_ui_token_amount_trimmed(amount: &UiTokenAmount) -> String {
    let s = stringify_ui_token_amount(amount);
    let zeros_trimmed = s.trim_end_matches('0');
    let decimal_trimmed = zeros_trimmed.trim_end_matches('.');
    decimal_trimmed.to_string()
}

fn command_account_info(config: &Config, address: Pubkey) -> CommandResult {
    let account = config.rpc_client.get_token_account(&address)?.unwrap();
    println!();
    println_name_value("Address:", &address.to_string());
    println_name_value(
        "Balance:",
        &stringify_ui_token_amount_trimmed(&account.token_amount),
    );
    let mint = format!(
        "{}{}",
        account.mint,
        if account.is_native { " (native)" } else { "" }
    );
    println_name_value("Mint:", &mint);
    println_name_value("Owner:", &account.owner);
    println_name_value("State:", &format!("{:?}", account.state));
    if let Some(delegate) = &account.delegate {
        println!("Delegation:");
        println_name_value("  Delegate:", delegate);
        let allowance = account.delegated_amount.as_ref().unwrap();
        println_name_value(
            "  Allowance:",
            &stringify_ui_token_amount_trimmed(&allowance),
        );
    } else {
        println_name_value("Delegation:", "");
    }
    println_name_value(
        "Close authority:",
        &account.close_authority.as_ref().unwrap_or(&String::new()),
    );
    Ok(None)
}

fn get_multisig(config: &Config, address: &Pubkey) -> Result<Multisig, Error> {
    let account = config.rpc_client.get_account(&address)?;
    Multisig::unpack(&account.data).map_err(|e| e.into())
}

fn command_multisig(config: &Config, address: Pubkey) -> CommandResult {
    let multisig = get_multisig(config, &address)?;
    let n = multisig.n as usize;
    assert!(n <= multisig.signers.len());
    println!();
    println_name_value("Address:", &address.to_string());
    println_name_value("M/N:", &format!("{}/{}", multisig.m, n));
    println_name_value("Signers:", " ");
    let width = if n >= 9 { 4 } else { 3 };
    for i in 0..n {
        let title = format!("{1:>0$}:", width, i + 1);
        let pubkey = &multisig.signers[i];
        println_name_value(&title, &pubkey.to_string())
    }
    Ok(None)
}

struct SignOnlyNeedsFullMintSpec {}
impl offline::ArgsConfig for SignOnlyNeedsFullMintSpec {
    fn sign_only_arg<'a, 'b>(&self, arg: Arg<'a, 'b>) -> Arg<'a, 'b> {
        arg.requires_all(&[MINT_ADDRESS_ARG.name, MINT_DECIMALS_ARG.name])
    }
}

struct SignOnlyNeedsMintDecimals {}
impl offline::ArgsConfig for SignOnlyNeedsMintDecimals {
    fn sign_only_arg<'a, 'b>(&self, arg: Arg<'a, 'b>) -> Arg<'a, 'b> {
        arg.requires_all(&[MINT_DECIMALS_ARG.name])
    }
}

struct SignOnlyNeedsMintAddress {}
impl offline::ArgsConfig for SignOnlyNeedsMintAddress {
    fn sign_only_arg<'a, 'b>(&self, arg: Arg<'a, 'b>) -> Arg<'a, 'b> {
        arg.requires_all(&[MINT_ADDRESS_ARG.name])
    }
}

struct SignOnlyNeedsDelegateAddress {}
impl offline::ArgsConfig for SignOnlyNeedsDelegateAddress {
    fn sign_only_arg<'a, 'b>(&self, arg: Arg<'a, 'b>) -> Arg<'a, 'b> {
        arg.requires_all(&[DELEGATE_ADDRESS_ARG.name])
    }
}

fn main() {
    let default_decimals = &format!("{}", native_mint::DECIMALS);
    let app_matches = App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg({
            let arg = Arg::with_name("config_file")
                .short("C")
                .long("config")
                .value_name("PATH")
                .takes_value(true)
                .global(true)
                .help("Configuration file to use");
            if let Some(ref config_file) = *solana_cli_config::CONFIG_FILE {
                arg.default_value(&config_file)
            } else {
                arg
            }
        })
        .arg(
            Arg::with_name("verbose")
                .long("verbose")
                .short("v")
                .takes_value(false)
                .global(true)
                .help("Show additional information"),
        )
        .arg(
            Arg::with_name("json_rpc_url")
                .long("url")
                .value_name("URL")
                .takes_value(true)
                .global(true)
                .validator(is_url)
                .help("JSON RPC URL for the cluster.  Default from the configuration file."),
        )
        .arg(
            Arg::with_name("owner")
                .long("owner")
                .value_name("KEYPAIR")
                .validator(is_valid_signer)
                .takes_value(true)
                .global(true)
                .help(
                    "Specify the token owner account. \
                     This may be a keypair file, the ASK keyword. \
                     Defaults to the client keypair.",
                ),
        )
        .arg(fee_payer_arg().global(true))
        .subcommand(SubCommand::with_name("create-token").about("Create a new token")
                .arg(
                    Arg::with_name("decimals")
                        .long("decimals")
                        .validator(is_mint_decimals)
                        .value_name("DECIMALS")
                        .takes_value(true)
                        .default_value(&default_decimals)
                        .help("Number of base 10 digits to the right of the decimal place"),
                )
                .arg(
                    Arg::with_name("token_keypair")
                        .value_name("KEYPAIR")
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .index(1)
                        .help(
                            "Specify the token keypair. \
                             This may be a keypair file or the ASK keyword. \
                             [default: randomly generated keypair]"
                        ),
                )
                .arg(
                    Arg::with_name("enable_freeze")
                        .long("enable-freeze")
                        .takes_value(false)
                        .help(
                            "Enable the mint authority to freeze associated token accounts."
                        ),
                )
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name("create-account")
                .about("Create a new token account")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token that the account will hold"),
                )
                .arg(
                    Arg::with_name("account_keypair")
                        .value_name("KEYPAIR")
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .index(2)
                        .help(
                            "Specify the account keypair. \
                             This may be a keypair file or the ASK keyword. \
                             [default: associated token account for --owner]"
                        ),
                )
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name("create-multisig")
                .about("Create a new account describing an M:N multisignature")
                .arg(
                    Arg::with_name("minimum_signers")
                        .value_name("MINIMUM_SIGNERS")
                        .validator(is_multisig_minimum_signers)
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help(&format!("The minimum number of signers required \
                            to allow the operation. [{} <= M <= N]",
                            MIN_SIGNERS,
                        )),
                )
                .arg(
                    Arg::with_name("multisig_member")
                        .value_name("MULTISIG_MEMBER_PUBKEY")
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .min_values(MIN_SIGNERS as u64)
                        .max_values(MAX_SIGNERS as u64)
                        .help(&format!("The public keys for each of the N \
                            signing members of this account. [{} <= N <= {}]",
                            MIN_SIGNERS, MAX_SIGNERS,
                        )),
                )
                .arg(
                    Arg::with_name("address_keypair")
                        .long("address-keypair")
                        .value_name("ADDRESS_KEYPAIR")
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help(
                            "Specify the address keypair. \
                             This may be a keypair file or the ASK keyword. \
                             [default: randomly generated keypair]"
                        ),
                )
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name("authorize")
                .about("Authorize a new signing keypair to a token or token account")
                .arg(
                    Arg::with_name("address")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account"),
                )
                .arg(
                    Arg::with_name("authority_type")
                        .value_name("AUTHORITY_TYPE")
                        .takes_value(true)
                        .possible_values(&["mint", "freeze", "owner", "close"])
                        .index(2)
                        .required(true)
                        .help("The new authority type. \
                            Token mints support `mint` and `freeze` authorities;\
                            Token accounts support `owner` and `close` authorities."),
                )
                .arg(
                    Arg::with_name("new_authority")
                        .validator(is_valid_pubkey)
                        .value_name("AUTHORITY_ADDRESS")
                        .takes_value(true)
                        .index(3)
                        .required_unless("disable")
                        .help("The address of the new authority"),
                )
                .arg(
                    Arg::with_name("disable")
                        .long("disable")
                        .takes_value(false)
                        .conflicts_with("new_authority")
                        .help("Disable mint, freeze, or close functionality by setting authority to None.")
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name("transfer")
                .about("Transfer tokens between accounts")
                .arg(
                    Arg::with_name("sender")
                        .validator(is_valid_pubkey)
                        .value_name("SENDER_TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token account address of the sender"),
                )
                .arg(
                    Arg::with_name("amount")
                        .validator(is_amount_or_all)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to send, in tokens; accepts keyword ALL"),
                )
                .arg(
                    Arg::with_name("recipient")
                        .validator(is_valid_pubkey)
                        .value_name("RECIPIENT_ADDRESS or RECIPIENT_TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(3)
                        .required(true)
                        .help("If a token account address is provided, use it as the recipient. \
                               Otherwise assume the recipient address is a user wallet and transfer to \
                               the associated token account")
                )
                .arg(
                    Arg::with_name("fund_recipient")
                        .long("fund-recipient")
                        .takes_value(false)
                        .help("Create the associated token account for the recipient if doesn't already exist")
                )
                .arg(multisig_signer_arg())
                .mint_args()
                .nonce_args(true)
                .offline_args_config(&SignOnlyNeedsFullMintSpec{}),
        )
        .subcommand(
            SubCommand::with_name("burn")
                .about("Burn tokens from an account")
                .arg(
                    Arg::with_name("source")
                        .validator(is_valid_pubkey)
                        .value_name("SOURCE_TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token account address to burn from"),
                )
                .arg(
                    Arg::with_name("amount")
                        .validator(is_amount)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to burn, in tokens"),
                )
                .arg(multisig_signer_arg())
                .mint_args()
                .nonce_args(true)
                .offline_args_config(&SignOnlyNeedsFullMintSpec{}),
        )
        .subcommand(
            SubCommand::with_name("mint")
                .about("Mint new tokens")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token to mint"),
                )
                .arg(
                    Arg::with_name("amount")
                        .validator(is_amount)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to mint, in tokens"),
                )
                .arg(
                    Arg::with_name("recipient")
                        .validator(is_valid_pubkey)
                        .value_name("RECIPIENT_TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(3)
                        .help("The token account address of recipient [default: associated token account for --owner]"),
                )
                .arg(mint_decimals_arg())
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args_config(&SignOnlyNeedsMintDecimals{}),
        )
        .subcommand(
            SubCommand::with_name("freeze")
                .about("Freeze a token account")
                .arg(
                    Arg::with_name("account")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to freeze"),
                )
                .arg(mint_address_arg())
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args_config(&SignOnlyNeedsMintAddress{}),
        )
        .subcommand(
            SubCommand::with_name("thaw")
                .about("Thaw a token account")
                .arg(
                    Arg::with_name("account")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to thaw"),
                )
                .arg(mint_address_arg())
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args_config(&SignOnlyNeedsMintAddress{}),
        )
        .subcommand(
            SubCommand::with_name("balance")
                .about("Get token account balance")
                .arg(
                    Arg::with_name("address")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token account address"),
                ),
        )
        .subcommand(
            SubCommand::with_name("supply")
                .about("Get token supply")
                .arg(
                    Arg::with_name("address")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token address"),
                ),
        )
        .subcommand(
            SubCommand::with_name("accounts")
                .about("List all token accounts by owner")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .help("Limit results to the given token. [Default: list accounts for all tokens]"),
                ),
        )
        .subcommand(
            SubCommand::with_name("wrap")
                .about("Wrap native SOL in a SOL token account")
                .arg(
                    Arg::with_name("amount")
                        .validator(is_amount)
                        .value_name("AMOUNT")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("Amount of SOL to wrap"),
                )
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name("unwrap")
                .about("Unwrap a SOL token account")
                .arg(
                    Arg::with_name("address")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to unwrap"),
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name("account-info")
                .about("Query details of an SPL Token account by address")
                .arg(
                    Arg::with_name("address")
                    .validator(is_valid_pubkey)
                    .value_name("TOKEN_ACCOUNT_ADDRESS")
                    .takes_value(true)
                    .index(1)
                    .required(true)
                    .help("The address of the SPL Token account to query"),
                ),
        )
        .subcommand(
            SubCommand::with_name("multisig-info")
                .about("Query details about and SPL Token multisig account by address")
                .arg(
                    Arg::with_name("address")
                    .validator(is_valid_pubkey)
                    .value_name("MULTISIG_ACCOUNT_ADDRESS")
                    .takes_value(true)
                    .index(1)
                    .required(true)
                    .help("The address of the SPL Token multisig account to query"),
                ),
        )
        .subcommand(
            SubCommand::with_name("approve")
                .about("Approve a delegate for a token account")
                .arg(
                    Arg::with_name("account")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to delegate"),
                )
                .arg(
                    Arg::with_name("amount")
                        .validator(is_amount)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to approve, in tokens"),
                )
                .arg(
                    Arg::with_name("delegate")
                        .validator(is_valid_pubkey)
                        .value_name("DELEGATE_TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(3)
                        .required(true)
                        .help("The token account address of delegate"),
                )
                .arg(multisig_signer_arg())
                .mint_args()
                .nonce_args(true)
                .offline_args_config(&SignOnlyNeedsFullMintSpec{}),
        )
        .subcommand(
            SubCommand::with_name("revoke")
                .about("Revoke a delegate's authority")
                .arg(
                    Arg::with_name("account")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account"),
                )
                .arg(delegate_address_arg())
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args_config(&SignOnlyNeedsDelegateAddress{}),
        )
        .subcommand(
            SubCommand::with_name("close")
                .about("Close a token account")
                .arg(
                    Arg::with_name("account")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to close"),
                )
                .arg(
                    Arg::with_name("destination")
                        .validator(is_valid_pubkey)
                        .value_name("REFUND_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(2)
                        .help("The address of the account to receive remaining SOL [default: --owner]"),
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name("gc")
                .about("Cleanup unnecessary token accounts")
        )
        .get_matches();

    let mut wallet_manager = None;
    let mut bulk_signers: Vec<Option<Box<dyn Signer>>> = Vec::new();
    let mut multisigner_ids = Vec::new();

    let (sub_command, sub_matches) = app_matches.subcommand();
    let matches = sub_matches.unwrap();

    let config = {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };
        let json_rpc_url = matches
            .value_of("json_rpc_url")
            .unwrap_or(&cli_config.json_rpc_url)
            .to_string();

        let default_signer_arg_name = "owner".to_string();
        let default_signer_path = matches
            .value_of(&default_signer_arg_name)
            .map(|s| s.to_string())
            .unwrap_or_else(|| cli_config.keypair_path.clone());
        let default_signer = DefaultSigner {
            path: default_signer_path,
            arg_name: default_signer_arg_name,
        };
        // Owner doesn't sign when it's a multisig
        let owner = if matches.is_present(MULTISIG_SIGNER_ARG.name) {
            let owner_val = matches
                .value_of("owner")
                .unwrap_or(&cli_config.keypair_path);
            pubkey_from_path(&matches, owner_val, "owner", &mut wallet_manager).unwrap_or_else(
                |e| {
                    eprintln!("error: {}", e);
                    exit(1);
                },
            )
        } else {
            bulk_signers.push(None);
            default_signer
                .signer_from_path(&matches, &mut wallet_manager)
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    exit(1);
                })
                .pubkey()
        };

        let (signer, fee_payer) = signer_from_path(
            &matches,
            matches
                .value_of("fee_payer")
                .unwrap_or(&cli_config.keypair_path),
            "fee_payer",
            &mut wallet_manager,
        )
        .map(|s| {
            let p = s.pubkey();
            (Some(s), p)
        })
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            exit(1);
        });
        bulk_signers.push(signer);

        let verbose = matches.is_present("verbose");

        let nonce_account = pubkey_of_signer(&matches, NONCE_ARG.name, &mut wallet_manager)
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                exit(1);
            });
        let (signer, nonce_authority) =
            signer_of(&matches, NONCE_AUTHORITY_ARG.name, &mut wallet_manager).unwrap_or_else(
                |e| {
                    eprintln!("error: {}", e);
                    exit(1);
                },
            );
        if signer.is_some() {
            bulk_signers.push(signer);
        }

        let blockhash_query = BlockhashQuery::new_from_matches(matches);
        let sign_only = matches.is_present(SIGN_ONLY_ARG.name);

        let multisig_signers = signers_of(&matches, MULTISIG_SIGNER_ARG.name, &mut wallet_manager)
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                exit(1);
            });
        if let Some(multisig_signers) = multisig_signers {
            let (signers, pubkeys): (Vec<_>, Vec<_>) = multisig_signers.into_iter().unzip();
            bulk_signers.extend(signers.into_iter().map(Some));
            multisigner_ids = pubkeys;
        }
        let multisigner_pubkeys = multisigner_ids.iter().collect::<Vec<_>>();

        Config {
            rpc_client: RpcClient::new_with_commitment(json_rpc_url, CommitmentConfig::confirmed()),
            verbose,
            owner,
            fee_payer,
            default_signer,
            nonce_account,
            nonce_authority,
            blockhash_query,
            sign_only,
            multisigner_pubkeys,
        }
    };

    if matches.is_present(MULTISIG_SIGNER_ARG.name)
        && !config.sign_only
        && get_multisig(&config, &config.owner).is_err()
    {
        eprintln!("error: {} is not a multisig account", config.owner);
        exit(1);
    }

    solana_logger::setup_with_default("solana=info");

    let _ = match (sub_command, sub_matches) {
        ("create-token", Some(arg_matches)) => {
            let decimals = value_t_or_exit!(arg_matches, "decimals", u8);
            let (signer, token) = if arg_matches.is_present("token_keypair") {
                signer_of(&arg_matches, "token_keypair", &mut wallet_manager).unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    exit(1);
                })
            } else {
                new_throwaway_signer()
            };
            let token = token.unwrap();
            bulk_signers.push(signer);

            command_create_token(
                &config,
                decimals,
                token,
                arg_matches.is_present("enable_freeze"),
            )
        }
        ("create-account", Some(arg_matches)) => {
            let token = pubkey_of_signer(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();

            let (signer, account) = if arg_matches.is_present("account_keypair") {
                signer_of(&arg_matches, "account_keypair", &mut wallet_manager).unwrap_or_else(
                    |e| {
                        eprintln!("error: {}", e);
                        exit(1);
                    },
                )
            } else {
                (None, None)
            };
            bulk_signers.push(signer);

            command_create_account(&config, token, account)
        }
        ("create-multisig", Some(arg_matches)) => {
            let minimum_signers = value_of::<u8>(&arg_matches, "minimum_signers").unwrap();
            let multisig_members =
                pubkeys_of_multiple_signers(&arg_matches, "multisig_member", &mut wallet_manager)
                    .unwrap_or_else(|e| {
                        eprintln!("error: {}", e);
                        exit(1);
                    })
                    .unwrap();
            if minimum_signers as usize > multisig_members.len() {
                eprintln!(
                    "error: MINIMUM_SIGNERS cannot be greater than the number \
                          of MULTISIG_MEMBERs passed"
                );
                exit(1);
            }

            let (signer, account) = if arg_matches.is_present("address_keypair") {
                signer_of(&arg_matches, "address_keypair", &mut wallet_manager).unwrap_or_else(
                    |e| {
                        eprintln!("error: {}", e);
                        exit(1);
                    },
                )
            } else {
                new_throwaway_signer()
            };
            let account = account.unwrap();
            bulk_signers.push(signer);

            command_create_multisig(&config, account, minimum_signers, multisig_members)
        }
        ("authorize", Some(arg_matches)) => {
            let address = pubkey_of_signer(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let authority_type = arg_matches.value_of("authority_type").unwrap();
            let authority_type = match authority_type {
                "mint" => AuthorityType::MintTokens,
                "freeze" => AuthorityType::FreezeAccount,
                "owner" => AuthorityType::AccountOwner,
                "close" => AuthorityType::CloseAccount,
                _ => unreachable!(),
            };
            let new_authority =
                pubkey_of_signer(arg_matches, "new_authority", &mut wallet_manager).unwrap();
            command_authorize(&config, address, authority_type, new_authority)
        }
        ("transfer", Some(arg_matches)) => {
            let sender = pubkey_of_signer(arg_matches, "sender", &mut wallet_manager)
                .unwrap()
                .unwrap();

            let amount = match matches.value_of("amount").unwrap() {
                "ALL" => None,
                amount => Some(amount.parse::<f64>().unwrap()),
            };
            let recipient = pubkey_of_signer(arg_matches, "recipient", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let mint_address =
                pubkey_of_signer(arg_matches, MINT_ADDRESS_ARG.name, &mut wallet_manager).unwrap();
            let mint_decimals = value_of::<u8>(&arg_matches, MINT_DECIMALS_ARG.name);
            let fund_recipient = matches.is_present("fund_recipient");
            command_transfer(
                &config,
                sender,
                amount,
                recipient,
                fund_recipient,
                mint_address,
                mint_decimals,
            )
        }
        ("burn", Some(arg_matches)) => {
            let source = pubkey_of_signer(arg_matches, "source", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            let mint_address =
                pubkey_of_signer(arg_matches, MINT_ADDRESS_ARG.name, &mut wallet_manager).unwrap();
            let mint_decimals = value_of::<u8>(&arg_matches, MINT_DECIMALS_ARG.name);
            command_burn(&config, source, amount, mint_address, mint_decimals)
        }
        ("mint", Some(arg_matches)) => {
            let token = pubkey_of_signer(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            let recipient = pubkey_of_signer(arg_matches, "recipient", &mut wallet_manager)
                .unwrap()
                .unwrap_or_else(|| get_associated_token_address(&config.owner, &token));
            let mint_decimals = value_of::<u8>(&arg_matches, MINT_DECIMALS_ARG.name);
            command_mint(&config, token, amount, recipient, mint_decimals)
        }
        ("freeze", Some(arg_matches)) => {
            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let mint_address =
                pubkey_of_signer(arg_matches, MINT_ADDRESS_ARG.name, &mut wallet_manager).unwrap();
            command_freeze(&config, account, mint_address)
        }
        ("thaw", Some(arg_matches)) => {
            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let mint_address =
                pubkey_of_signer(arg_matches, MINT_ADDRESS_ARG.name, &mut wallet_manager).unwrap();
            command_thaw(&config, account, mint_address)
        }
        ("wrap", Some(arg_matches)) => {
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            let (signer, account) = new_throwaway_signer();
            let account = account.unwrap();
            bulk_signers.push(signer);
            command_wrap(&config, amount, account)
        }
        ("unwrap", Some(arg_matches)) => {
            let address = pubkey_of_signer(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_unwrap(&config, address)
        }
        ("approve", Some(arg_matches)) => {
            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            let delegate = pubkey_of_signer(arg_matches, "delegate", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let mint_address =
                pubkey_of_signer(arg_matches, MINT_ADDRESS_ARG.name, &mut wallet_manager).unwrap();
            let mint_decimals = value_of::<u8>(&arg_matches, MINT_DECIMALS_ARG.name);
            command_approve(
                &config,
                account,
                amount,
                delegate,
                mint_address,
                mint_decimals,
            )
        }
        ("revoke", Some(arg_matches)) => {
            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let delegate_address =
                pubkey_of_signer(arg_matches, DELEGATE_ADDRESS_ARG.name, &mut wallet_manager)
                    .unwrap();
            command_revoke(&config, account, delegate_address)
        }
        ("close", Some(arg_matches)) => {
            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let destination = pubkey_of_signer(arg_matches, "destination", &mut wallet_manager)
                .unwrap()
                .unwrap_or(config.owner);
            command_close(&config, account, destination)
        }
        ("balance", Some(arg_matches)) => {
            let address = pubkey_of_signer(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_balance(&config, address)
        }
        ("supply", Some(arg_matches)) => {
            let address = pubkey_of_signer(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_supply(&config, address)
        }
        ("accounts", Some(arg_matches)) => {
            let token = pubkey_of_signer(arg_matches, "token", &mut wallet_manager).unwrap();
            command_accounts(&config, token)
        }
        ("account-info", Some(arg_matches)) => {
            let address = pubkey_of_signer(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_account_info(&config, address)
        }
        ("multisig-info", Some(arg_matches)) => {
            let address = pubkey_of_signer(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_multisig(&config, address)
        }
        ("gc", Some(_arg_matches)) => command_gc(&config),
        _ => unreachable!(),
    }
    .and_then(|transaction_info| {
        if let Some((minimum_balance_for_rent_exemption, instruction_batches)) = transaction_info {
            let fee_payer = Some(&config.fee_payer);
            let signer_info = config
                .default_signer
                .generate_unique_signers(bulk_signers, &matches, &mut wallet_manager)
                .unwrap_or_else(|e| {
                    eprintln!("error: {}", e);
                    exit(1);
                });

            for instructions in instruction_batches {
                let message = if let Some(nonce_account) = config.nonce_account.as_ref() {
                    Message::new_with_nonce(
                        instructions,
                        fee_payer,
                        nonce_account,
                        config.nonce_authority.as_ref().unwrap(),
                    )
                } else {
                    Message::new(&instructions, fee_payer)
                };
                let (recent_blockhash, fee_calculator) = config
                    .blockhash_query
                    .get_blockhash_and_fee_calculator(
                        &config.rpc_client,
                        config.rpc_client.commitment(),
                    )
                    .unwrap_or_else(|e| {
                        eprintln!("error: {}", e);
                        exit(1);
                    });

                if !config.sign_only {
                    check_fee_payer_balance(
                        &config,
                        minimum_balance_for_rent_exemption + fee_calculator.calculate_fee(&message),
                    )?;
                }

                let mut transaction = Transaction::new_unsigned(message);

                if config.sign_only {
                    transaction.try_partial_sign(&signer_info.signers, recent_blockhash)?;
                    println!("{}", return_signers(&transaction, &OutputFormat::Display)?);
                } else {
                    transaction.try_sign(&signer_info.signers, recent_blockhash)?;
                    let signature = config
                        .rpc_client
                        .send_and_confirm_transaction_with_spinner(&transaction)?;
                    println!("Signature: {}", signature);
                }
            }
        }
        Ok(())
    })
    .map_err(|err| {
        eprintln!("{}", err);
        exit(1);
    });
}
