#![allow(clippy::integer_arithmetic)]
use clap::{
    crate_description, crate_name, crate_version, value_t, value_t_or_exit, App, AppSettings, Arg,
    ArgMatches, SubCommand,
};
use serde::Serialize;
use solana_account_decoder::{
    parse_token::{get_token_account_mint, parse_token, TokenAccountType, UiAccountState},
    UiAccountData,
};
use solana_clap_utils::{
    fee_payer::fee_payer_arg,
    input_parsers::{pubkey_of_signer, pubkeys_of_multiple_signers, value_of},
    input_validators::{
        is_amount, is_amount_or_all, is_parsable, is_pubkey, is_url_or_moniker, is_valid_pubkey,
        is_valid_signer,
    },
    keypair::signer_from_path,
    memo::memo_arg,
    nonce::*,
    offline::{self, *},
    ArgConstant,
};
use solana_cli_output::{
    return_signers_data, CliSignOnlyData, CliSignature, OutputFormat, QuietDisplay,
    ReturnSignersConfig, VerboseDisplay,
};
use solana_client::rpc_request::TokenAccountsFilter;
use solana_remote_wallet::remote_wallet::RemoteWalletManager;
use solana_sdk::{
    instruction::Instruction,
    message::Message,
    native_token::*,
    program_option::COption,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address_with_program_id, instruction::create_associated_token_account,
};
use spl_token_2022::{
    extension::{
        interest_bearing_mint::InterestBearingConfig, memo_transfer::MemoTransfer,
        mint_close_authority::MintCloseAuthority, ExtensionType, StateWithExtensionsOwned,
    },
    instruction::*,
    state::{Account, Mint},
};
use spl_token_client::{
    client::{ProgramRpcClientSendTransaction, RpcClientResponse},
    token::{ExtensionInitializationParams, Token},
};
use std::{collections::HashMap, fmt, fmt::Display, process::exit, str::FromStr, sync::Arc};
use strum_macros::{EnumString, IntoStaticStr};

mod config;
use config::{Config, MintInfo};

mod output;
use output::*;

mod sort;
use sort::{is_supported_program, sort_and_parse_token_accounts};

mod bench;
use bench::*;
use spl_token_2022::generic_token_account::GenericTokenAccount;

struct CliSignerInfo {
    pub signers: Vec<Arc<dyn Signer>>,
}

impl CliSignerInfo {
    pub fn signers_for_message(&self, message: &Message) -> Vec<&dyn Signer> {
        self.signers
            .iter()
            .filter_map(|k| {
                if message.signer_keys().contains(&&k.pubkey()) {
                    Some(k.as_ref())
                } else {
                    None
                }
            })
            .collect()
    }
}

pub const OWNER_ADDRESS_ARG: ArgConstant<'static> = ArgConstant {
    name: "owner",
    long: "owner",
    help: "Address of the token's owner. Defaults to the client keypair address.",
};

pub const OWNER_KEYPAIR_ARG: ArgConstant<'static> = ArgConstant {
    name: "owner",
    long: "owner",
    help: "Keypair of the token's owner. Defaults to the client keypair.",
};

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

#[derive(Debug, Clone, Copy, PartialEq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum CommandName {
    CreateToken,
    Close,
    CloseMint,
    Bench,
    CreateAccount,
    CreateMultisig,
    Authorize,
    SetInterestRate,
    Transfer,
    Burn,
    Mint,
    Freeze,
    Thaw,
    Wrap,
    Unwrap,
    Approve,
    Revoke,
    Balance,
    Supply,
    Accounts,
    Address,
    AccountInfo,
    MultisigInfo,
    Display,
    Gc,
    SyncNative,
    EnableRequiredTransferMemos,
    DisableRequiredTransferMemos,
}
impl fmt::Display for CommandName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn owner_address_arg<'a, 'b>() -> Arg<'a, 'b> {
    Arg::with_name(OWNER_ADDRESS_ARG.name)
        .long(OWNER_ADDRESS_ARG.long)
        .takes_value(true)
        .value_name("OWNER_ADDRESS")
        .validator(is_valid_pubkey)
        .help(OWNER_ADDRESS_ARG.help)
}

pub fn owner_keypair_arg_with_value_name<'a, 'b>(value_name: &'static str) -> Arg<'a, 'b> {
    Arg::with_name(OWNER_KEYPAIR_ARG.name)
        .long(OWNER_KEYPAIR_ARG.long)
        .takes_value(true)
        .value_name(value_name)
        .validator(is_valid_signer)
        .help(OWNER_KEYPAIR_ARG.help)
}

pub fn owner_keypair_arg<'a, 'b>() -> Arg<'a, 'b> {
    owner_keypair_arg_with_value_name("OWNER_KEYPAIR")
}

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

pub(crate) type Error = Box<dyn std::error::Error + Send + Sync>;

type BulkSigners = Vec<Arc<dyn Signer>>;
pub(crate) type CommandResult = Result<String, Error>;

fn new_throwaway_signer() -> (Arc<dyn Signer>, Pubkey) {
    let keypair = Keypair::new();
    let pubkey = keypair.pubkey();
    (Arc::new(keypair) as Arc<dyn Signer>, pubkey)
}

fn get_signer(
    matches: &ArgMatches<'_>,
    keypair_name: &str,
    wallet_manager: &mut Option<Arc<RemoteWalletManager>>,
) -> Option<(Arc<dyn Signer>, Pubkey)> {
    matches.value_of(keypair_name).map(|path| {
        let signer =
            signer_from_path(matches, path, keypair_name, wallet_manager).unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                exit(1);
            });
        let signer_pubkey = signer.pubkey();
        (Arc::from(signer), signer_pubkey)
    })
}

pub(crate) async fn check_fee_payer_balance(
    config: &Config<'_>,
    required_balance: u64,
) -> Result<(), Error> {
    let balance = config
        .rpc_client
        .get_balance(&config.fee_payer.pubkey())
        .await?;
    if balance < required_balance {
        Err(format!(
            "Fee payer, {}, has insufficient balance: {} required, {} available",
            config.fee_payer.pubkey(),
            lamports_to_sol(required_balance),
            lamports_to_sol(balance)
        )
        .into())
    } else {
        Ok(())
    }
}

async fn check_wallet_balance(
    config: &Config<'_>,
    wallet: &Pubkey,
    required_balance: u64,
) -> Result<(), Error> {
    let balance = config.rpc_client.get_balance(wallet).await?;
    if balance < required_balance {
        Err(format!(
            "Wallet {}, has insufficient balance: {} required, {} available",
            wallet,
            lamports_to_sol(required_balance),
            lamports_to_sol(balance)
        )
        .into())
    } else {
        Ok(())
    }
}

type SignersOf = Vec<(Arc<dyn Signer>, Pubkey)>;
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
            results.push((Arc::from(signer), signer_pubkey));
        }
        Ok(Some(results))
    } else {
        Ok(None)
    }
}

fn token_client_from_config(
    config: &Config<'_>,
    token_pubkey: &Pubkey,
) -> Token<ProgramRpcClientSendTransaction> {
    let token = Token::new(
        config.program_client.clone(),
        &config.program_id,
        token_pubkey,
        config.fee_payer.clone(),
    );

    if let (Some(nonce_account), Some(nonce_authority)) =
        (config.nonce_account, config.nonce_authority)
    {
        token.with_nonce(&nonce_account, &nonce_authority)
    } else {
        token
    }
}

fn native_token_client_from_config(config: &Config<'_>) -> Token<ProgramRpcClientSendTransaction> {
    let token = Token::new_native(
        config.program_client.clone(),
        &config.program_id,
        config.fee_payer.clone(),
    );

    if let (Some(nonce_account), Some(nonce_authority)) =
        (config.nonce_account, config.nonce_authority)
    {
        token.with_nonce(&nonce_account, &nonce_authority)
    } else {
        token
    }
}

#[allow(clippy::too_many_arguments)]
async fn command_create_token(
    config: &Config<'_>,
    decimals: u8,
    token_pubkey: Pubkey,
    authority: Pubkey,
    enable_freeze: bool,
    enable_close: bool,
    memo: Option<String>,
    rate_bps: Option<i16>,
    bulk_signers: Vec<Arc<dyn Signer>>,
) -> CommandResult {
    println_display(
        config,
        format!(
            "Creating token {} under program {}",
            token_pubkey, config.program_id
        ),
    );

    let token = token_client_from_config(config, &token_pubkey);

    let freeze_authority = if enable_freeze { Some(authority) } else { None };

    let mut extensions = vec![];

    if enable_close {
        extensions.push(ExtensionInitializationParams::MintCloseAuthority {
            close_authority: Some(authority),
        });
    }

    if let Some(rate_bps) = rate_bps {
        extensions.push(ExtensionInitializationParams::InterestBearingConfig {
            rate_authority: Some(authority),
            rate: rate_bps,
        })
    }

    if let Some(text) = memo {
        token.with_memo(text, vec![config.default_signer.pubkey()]);
    }

    let res = token
        .create_mint(
            &authority,
            freeze_authority.as_ref(),
            decimals,
            extensions,
            &bulk_signers,
        )
        .await?;

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(cli_signature) => format_output(
            CliCreateToken {
                address: token_pubkey.to_string(),
                decimals,
                transaction_data: cli_signature,
            },
            &CommandName::CreateToken,
            config,
        ),
        TransactionReturnData::CliSignOnlyData(cli_sign_only_data) => {
            format_output(cli_sign_only_data, &CommandName::CreateToken, config)
        }
    })
}

async fn command_set_interest_rate(
    config: &Config<'_>,
    token_pubkey: Pubkey,
    rate_authority: Pubkey,
    rate_bps: i16,
    bulk_signers: Vec<Arc<dyn Signer>>,
) -> CommandResult {
    let token = token_client_from_config(config, &token_pubkey);

    if !config.sign_only {
        let mint_account = config.get_account_checked(&token_pubkey).await?;

        let mint_state = StateWithExtensionsOwned::<Mint>::unpack(mint_account.data)
            .map_err(|_| format!("Could not deserialize token mint {}", token_pubkey))?;

        if let Ok(interest_rate_config) = mint_state.get_extension::<InterestBearingConfig>() {
            let mint_rate_authority_pubkey =
                Option::<Pubkey>::from(interest_rate_config.rate_authority);

            if mint_rate_authority_pubkey != Some(rate_authority) {
                return Err(format!(
                    "Mint {} has interest rate authority {}, but {} was provided",
                    token_pubkey,
                    mint_rate_authority_pubkey
                        .map(|pubkey| pubkey.to_string())
                        .unwrap_or_else(|| "disabled".to_string()),
                    rate_authority
                )
                .into());
            }
        } else {
            return Err(format!("Mint {} is not interest-bearing", token_pubkey).into());
        }
    }

    println_display(
        config,
        format!(
            "Setting Interest Rate for {} to {} bps",
            token_pubkey, rate_bps
        ),
    );

    let res = token
        .update_interest_rate(&rate_authority, rate_bps, &bulk_signers)
        .await?;

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

async fn command_create_account(
    config: &Config<'_>,
    token_pubkey: Pubkey,
    owner: Pubkey,
    maybe_account: Option<Pubkey>,
    immutable_owner: bool,
    bulk_signers: Vec<Arc<dyn Signer>>,
) -> CommandResult {
    let token = token_client_from_config(config, &token_pubkey);
    let mut extensions = vec![];

    let (account, is_associated) = if let Some(account) = maybe_account {
        (
            account,
            token.get_associated_token_address(&owner) == account,
        )
    } else {
        (token.get_associated_token_address(&owner), true)
    };

    println_display(config, format!("Creating account {}", account));

    if !config.sign_only {
        if let Some(account_data) = config.program_client.get_account(account).await? {
            if account_data.owner != system_program::id() || !is_associated {
                return Err(format!("Error: Account already exists: {}", account).into());
            }
        }
    }

    if immutable_owner {
        if config.program_id == spl_token::id() {
            return Err(format!(
                "Specified --immutable, but token program {} does not support the extension",
                config.program_id
            )
            .into());
        } else if is_associated {
            println_display(
                config,
                "Note: --immutable specified, but Token-2022 ATAs are always immutable, ignoring"
                    .to_string(),
            );
        } else {
            extensions.push(ExtensionType::ImmutableOwner);
        }
    }

    let res = if is_associated {
        token.create_associated_token_account(&owner).await
    } else {
        let signer = bulk_signers
            .iter()
            .find(|signer| signer.pubkey() == account)
            .unwrap_or_else(|| panic!("No signer provided for account {}", account));

        token
            .create_auxiliary_token_account_with_extension_space(&**signer, &owner, extensions)
            .await
    }?;

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

async fn command_create_multisig(
    config: &Config<'_>,
    multisig: Arc<dyn Signer>,
    minimum_signers: u8,
    multisig_members: Vec<Pubkey>,
) -> CommandResult {
    println_display(
        config,
        format!(
            "Creating {}/{} multisig {} under program {}",
            minimum_signers,
            multisig_members.len(),
            multisig.pubkey(),
            config.program_id,
        ),
    );

    // default is safe here because create_multisig doesnt use it
    let token = token_client_from_config(config, &Pubkey::default());

    let res = token
        .create_multisig(
            &*multisig,
            &multisig_members.iter().collect::<Vec<_>>(),
            minimum_signers,
        )
        .await?;

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

#[allow(clippy::too_many_arguments)]
async fn command_authorize(
    config: &Config<'_>,
    account: Pubkey,
    authority_type: AuthorityType,
    authority: Pubkey,
    new_authority: Option<Pubkey>,
    force_authorize: bool,
    bulk_signers: BulkSigners,
) -> CommandResult {
    let auth_str = match authority_type {
        AuthorityType::MintTokens => "mint authority",
        AuthorityType::FreezeAccount => "freeze authority",
        AuthorityType::AccountOwner => "owner",
        AuthorityType::CloseAccount => "close account authority",
        AuthorityType::CloseMint => "close mint authority",
        AuthorityType::TransferFeeConfig => "transfer fee authority",
        AuthorityType::WithheldWithdraw => "withdraw withheld authority",
        AuthorityType::InterestRate => "interest rate authority",
    };

    let (mint_pubkey, previous_authority) = if !config.sign_only {
        let target_account = config.get_account_checked(&account).await?;

        let (mint_pubkey, previous_authority) = if let Ok(mint) =
            StateWithExtensionsOwned::<Mint>::unpack(target_account.data.clone())
        {
            let previous_authority = match authority_type {
                AuthorityType::AccountOwner | AuthorityType::CloseAccount => Err(format!(
                    "Authority type `{}` not supported for SPL Token mints",
                    auth_str
                )),
                AuthorityType::MintTokens => Ok(mint.base.mint_authority),
                AuthorityType::FreezeAccount => Ok(mint.base.freeze_authority),
                AuthorityType::CloseMint => {
                    if let Ok(mint_close_authority) = mint.get_extension::<MintCloseAuthority>() {
                        Ok(COption::<Pubkey>::from(
                            mint_close_authority.close_authority,
                        ))
                    } else {
                        Err(format!(
                            "Mint `{}` does not support close authority",
                            account
                        ))
                    }
                }
                AuthorityType::TransferFeeConfig => unimplemented!(),
                AuthorityType::WithheldWithdraw => unimplemented!(),
                AuthorityType::InterestRate => {
                    if let Ok(interest_rate_config) = mint.get_extension::<InterestBearingConfig>()
                    {
                        Ok(COption::<Pubkey>::from(interest_rate_config.rate_authority))
                    } else {
                        Err(format!("Mint `{}` is not interest-bearing", account))
                    }
                }
            }?;

            Ok((account, previous_authority))
        } else if let Ok(token_account) =
            StateWithExtensionsOwned::<Account>::unpack(target_account.data)
        {
            let check_associated_token_account = || -> Result<(), Error> {
                let maybe_associated_token_account = get_associated_token_address_with_program_id(
                    &token_account.base.owner,
                    &token_account.base.mint,
                    &config.program_id,
                );
                if account == maybe_associated_token_account
                    && !force_authorize
                    && Some(authority) != new_authority
                {
                    Err(format!(
                        "Error: attempting to change the `{}` of an associated token account",
                        auth_str
                    )
                    .into())
                } else {
                    Ok(())
                }
            };

            let previous_authority = match authority_type {
                AuthorityType::MintTokens
                | AuthorityType::FreezeAccount
                | AuthorityType::CloseMint
                | AuthorityType::TransferFeeConfig
                | AuthorityType::WithheldWithdraw
                | AuthorityType::InterestRate => Err(format!(
                    "Authority type `{}` not supported for SPL Token accounts",
                    auth_str
                )),
                AuthorityType::AccountOwner => {
                    check_associated_token_account()?;
                    Ok(COption::Some(token_account.base.owner))
                }
                AuthorityType::CloseAccount => {
                    check_associated_token_account()?;
                    Ok(COption::Some(
                        token_account
                            .base
                            .close_authority
                            .unwrap_or(token_account.base.owner),
                    ))
                }
            }?;

            Ok((token_account.base.mint, previous_authority))
        } else {
            Err("Unsupported account data format".to_string())
        }?;

        (mint_pubkey, previous_authority)
    } else {
        // default is safe here because authorize doesnt use it
        (Pubkey::default(), COption::None)
    };

    let token = token_client_from_config(config, &mint_pubkey);

    println_display(
        config,
        format!(
            "Updating {}\n  Current {}: {}\n  New {}: {}",
            account,
            auth_str,
            previous_authority
                .map(|pubkey| pubkey.to_string())
                .unwrap_or_else(|| if config.sign_only {
                    "unknown".to_string()
                } else {
                    "disabled".to_string()
                }),
            auth_str,
            new_authority
                .map(|pubkey| pubkey.to_string())
                .unwrap_or_else(|| "disabled".to_string())
        ),
    );

    let res = token
        .set_authority(
            &account,
            &authority,
            new_authority.as_ref(),
            authority_type,
            &bulk_signers,
        )
        .await?;

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

async fn validate_mint(config: &Config<'_>, token: Pubkey) -> Result<Pubkey, Error> {
    let mint = config.get_account_checked(&token).await?;
    if StateWithExtensionsOwned::<Mint>::unpack(mint.data).is_err() {
        return Err(format!("Invalid mint account {:?}", token).into());
    }
    Ok(mint.owner)
}

#[allow(clippy::too_many_arguments)]
async fn command_transfer(
    config: &Config<'_>,
    token: Pubkey,
    ui_amount: Option<f64>,
    recipient: Pubkey,
    sender: Option<Pubkey>,
    sender_owner: Pubkey,
    allow_unfunded_recipient: bool,
    fund_recipient: bool,
    mint_decimals: Option<u8>,
    recipient_is_ata_owner: bool,
    use_unchecked_instruction: bool,
    memo: Option<String>,
    bulk_signers: BulkSigners,
    no_wait: bool,
    allow_non_system_account_recipient: bool,
) -> CommandResult {
    let mint_info = config.get_mint_info(&token, mint_decimals).await?;
    let sender = if let Some(sender) = sender {
        sender
    } else {
        get_associated_token_address_with_program_id(&sender_owner, &token, &mint_info.program_id)
    };
    config.check_account(&sender, Some(token)).await?;
    let maybe_transfer_balance =
        ui_amount.map(|ui_amount| spl_token::ui_amount_to_amount(ui_amount, mint_info.decimals));
    let transfer_balance = if !config.sign_only {
        let sender_token_amount = config
            .rpc_client
            .get_token_account_balance(&sender)
            .await
            .map_err(|err| {
                format!(
                    "Error: Failed to get token balance of sender address {}: {}",
                    sender, err
                )
            })?;
        let sender_balance = sender_token_amount.amount.parse::<u64>().map_err(|err| {
            format!(
                "Token account {} balance could not be parsed: {}",
                sender, err
            )
        })?;
        let transfer_balance = maybe_transfer_balance.unwrap_or(sender_balance);
        println_display(
            config,
            format!(
                "Transfer {} tokens\n  Sender: {}\n  Recipient: {}",
                spl_token::amount_to_ui_amount(transfer_balance, mint_info.decimals),
                sender,
                recipient
            ),
        );

        if transfer_balance > sender_balance {
            return Err(format!(
                "Error: Sender has insufficient funds, current balance is {}",
                spl_token_2022::amount_to_ui_amount_string_trimmed(
                    sender_balance,
                    mint_info.decimals
                )
            )
            .into());
        }
        transfer_balance
    } else {
        maybe_transfer_balance.unwrap()
    };

    let mut instructions = vec![];

    let mut recipient_token_account = recipient;
    let mut minimum_balance_for_rent_exemption = 0;

    let recipient_is_token_account = if !config.sign_only {
        let recipient_account_info = config
            .rpc_client
            .get_account_with_commitment(&recipient, config.rpc_client.commitment())
            .await?
            .value
            .map(|account| {
                (
                    account.owner == mint_info.program_id
                        && Account::valid_account_data(account.data.as_slice()),
                    account.owner == system_program::id(),
                )
            });
        if let Some((recipient_is_token_account, recipient_is_system_account)) =
            recipient_account_info
        {
            if !recipient_is_token_account
                && !recipient_is_system_account
                && !allow_non_system_account_recipient
            {
                return Err("Error: The recipient address is not owned by the System Program. \
                                     Add `--allow-non-system-account-recipient` to complete the transfer. \
                                    ".into());
            }
        } else if recipient_account_info.is_none() && !allow_unfunded_recipient {
            return Err("Error: The recipient address is not funded. \
                        Add `--allow-unfunded-recipient` to complete the transfer. \
                                   "
            .into());
        }
        recipient_account_info
            .map(|(recipient_is_token_account, _)| recipient_is_token_account)
            .unwrap_or(false)
    } else {
        !recipient_is_ata_owner
    };

    if !recipient_is_token_account {
        recipient_token_account = get_associated_token_address_with_program_id(
            &recipient,
            &mint_info.address,
            &mint_info.program_id,
        );
        println_display(
            config,
            format!(
                "  Recipient associated token account: {}",
                recipient_token_account
            ),
        );

        let needs_funding = if !config.sign_only {
            if let Some(recipient_token_account_data) = config
                .rpc_client
                .get_account_with_commitment(
                    &recipient_token_account,
                    config.rpc_client.commitment(),
                )
                .await?
                .value
            {
                if recipient_token_account_data.owner == system_program::id() {
                    true
                } else if recipient_token_account_data.owner == mint_info.program_id {
                    false
                } else {
                    return Err(
                        format!("Error: Unsupported recipient address: {}", recipient).into(),
                    );
                }
            } else {
                true
            }
        } else {
            fund_recipient
        };

        if needs_funding {
            if fund_recipient {
                if !config.sign_only {
                    minimum_balance_for_rent_exemption += config
                        .program_client
                        .get_minimum_balance_for_rent_exemption(Account::LEN)
                        .await?;
                    println_display(
                        config,
                        format!(
                            "  Funding recipient: {} ({} SOL)",
                            recipient_token_account,
                            lamports_to_sol(minimum_balance_for_rent_exemption)
                        ),
                    );
                }
                instructions.push(create_associated_token_account(
                    &config.fee_payer.pubkey(),
                    &recipient,
                    &mint_info.address,
                    &mint_info.program_id,
                ));
            } else {
                return Err(
                    "Error: Recipient's associated token account does not exist. \
                                    Add `--fund-recipient` to fund their account"
                        .into(),
                );
            }
        }
    }

    if use_unchecked_instruction {
        #[allow(deprecated)]
        instructions.push(transfer(
            &mint_info.program_id,
            &sender,
            &recipient_token_account,
            &sender_owner,
            &config.multisigner_pubkeys,
            transfer_balance,
        )?);
    } else {
        instructions.push(transfer_checked(
            &mint_info.program_id,
            &sender,
            &mint_info.address,
            &recipient_token_account,
            &sender_owner,
            &config.multisigner_pubkeys,
            transfer_balance,
            mint_info.decimals,
        )?);
    }
    if let Some(text) = memo {
        instructions.push(spl_memo::build_memo(
            text.as_bytes(),
            &[&config.fee_payer.pubkey()],
        ));
    }
    let tx_return = handle_tx(
        &CliSignerInfo {
            signers: bulk_signers,
        },
        config,
        no_wait,
        minimum_balance_for_rent_exemption,
        instructions,
    )
    .await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

#[allow(clippy::too_many_arguments)]
async fn command_burn(
    config: &Config<'_>,
    account: Pubkey,
    owner: Pubkey,
    ui_amount: f64,
    mint_address: Option<Pubkey>,
    mint_decimals: Option<u8>,
    use_unchecked_instruction: bool,
    memo: Option<String>,
    bulk_signers: BulkSigners,
) -> CommandResult {
    println_display(
        config,
        format!("Burn {} tokens\n  Source: {}", ui_amount, account),
    );

    let mint_address = config.check_account(&account, mint_address).await?;
    let mint_info = config.get_mint_info(&mint_address, mint_decimals).await?;
    let amount = spl_token::ui_amount_to_amount(ui_amount, mint_info.decimals);
    let decimals = if use_unchecked_instruction {
        None
    } else {
        Some(mint_info.decimals)
    };

    let token = token_client_from_config(config, &mint_info.address);
    if let Some(text) = memo {
        token.with_memo(text, vec![config.default_signer.pubkey()]);
    }

    let res = token
        .burn(&account, &owner, amount, decimals, &bulk_signers)
        .await?;

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

#[allow(clippy::too_many_arguments)]
async fn command_mint(
    config: &Config<'_>,
    token: Pubkey,
    ui_amount: f64,
    recipient: Pubkey,
    mint_info: MintInfo,
    mint_authority: Pubkey,
    use_unchecked_instruction: bool,
    memo: Option<String>,
    bulk_signers: BulkSigners,
) -> CommandResult {
    println_display(
        config,
        format!(
            "Minting {} tokens\n  Token: {}\n  Recipient: {}",
            ui_amount, token, recipient
        ),
    );

    let amount = spl_token::ui_amount_to_amount(ui_amount, mint_info.decimals);
    let decimals = if use_unchecked_instruction {
        None
    } else {
        Some(mint_info.decimals)
    };

    let token = token_client_from_config(config, &mint_info.address);
    if let Some(text) = memo {
        token.with_memo(text, vec![config.default_signer.pubkey()]);
    }

    let res = token
        .mint_to(&recipient, &mint_authority, amount, decimals, &bulk_signers)
        .await?;

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

async fn command_freeze(
    config: &Config<'_>,
    account: Pubkey,
    mint_address: Option<Pubkey>,
    freeze_authority: Pubkey,
    bulk_signers: BulkSigners,
) -> CommandResult {
    let mint_address = config.check_account(&account, mint_address).await?;
    let mint_info = config.get_mint_info(&mint_address, None).await?;

    println_display(
        config,
        format!(
            "Freezing account: {}\n  Token: {}",
            account, mint_info.address
        ),
    );

    let token = token_client_from_config(config, &mint_info.address);
    let res = token
        .freeze(&account, &freeze_authority, &bulk_signers)
        .await?;

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

async fn command_thaw(
    config: &Config<'_>,
    account: Pubkey,
    mint_address: Option<Pubkey>,
    freeze_authority: Pubkey,
    bulk_signers: BulkSigners,
) -> CommandResult {
    let mint_address = config.check_account(&account, mint_address).await?;
    let mint_info = config.get_mint_info(&mint_address, None).await?;

    println_display(
        config,
        format!(
            "Thawing account: {}\n  Token: {}",
            account, mint_info.address
        ),
    );

    let token = token_client_from_config(config, &mint_info.address);
    let res = token
        .thaw(&account, &freeze_authority, &bulk_signers)
        .await?;

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

async fn command_wrap(
    config: &Config<'_>,
    sol: f64,
    wallet_address: Pubkey,
    wrapped_sol_account: Option<Pubkey>,
    immutable_owner: bool,
    bulk_signers: BulkSigners,
) -> CommandResult {
    let lamports = sol_to_lamports(sol);
    let token = native_token_client_from_config(config);

    let account = wrapped_sol_account.unwrap_or_else(|| {
        get_associated_token_address_with_program_id(
            &wallet_address,
            token.get_address(),
            &config.program_id,
        )
    });

    println_display(config, format!("Wrapping {} SOL into {}", sol, account));

    if !config.sign_only {
        if let Some(account_data) = config.program_client.get_account(account).await? {
            if account_data.owner != system_program::id() {
                return Err(format!("Error: Account already exists: {}", account).into());
            }
        }

        check_wallet_balance(config, &wallet_address, lamports).await?;
    }

    let res = if immutable_owner {
        if config.program_id == spl_token::id() {
            return Err(format!(
                "Specified --immutable, but token program {} does not support the extension",
                config.program_id
            )
            .into());
        }

        token
            .wrap(&account, &wallet_address, lamports, &bulk_signers)
            .await?
    } else {
        // this case is hit for a token22 ata, which is always immutable. but it does the right thing anyway
        token
            .wrap_with_mutable_ownership(&account, &wallet_address, lamports, &bulk_signers)
            .await?
    };

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

async fn command_unwrap(
    config: &Config<'_>,
    wallet_address: Pubkey,
    maybe_account: Option<Pubkey>,
    bulk_signers: BulkSigners,
) -> CommandResult {
    let use_associated_account = maybe_account.is_none();
    let token = native_token_client_from_config(config);

    let account = maybe_account.unwrap_or_else(|| {
        get_associated_token_address_with_program_id(
            &wallet_address,
            token.get_address(),
            &config.program_id,
        )
    });

    println_display(config, format!("Unwrapping {}", account));

    if !config.sign_only {
        let account_data = config.get_account_checked(&account).await?;

        if !use_associated_account {
            let account_state = StateWithExtensionsOwned::<Account>::unpack(account_data.data)?;

            if account_state.base.mint != *token.get_address() {
                return Err(format!("{} is not a native token account", account).into());
            }
        }

        if account_data.lamports == 0 {
            if use_associated_account {
                return Err("No wrapped SOL in associated account; did you mean to specify an auxiliary address?".to_string().into());
            } else {
                return Err(format!("No wrapped SOL in {}", account).into());
            }
        }

        println_display(
            config,
            format!("  Amount: {} SOL", lamports_to_sol(account_data.lamports)),
        );
    }

    println_display(config, format!("  Recipient: {}", &wallet_address));

    let res = token
        .close_account(&account, &wallet_address, &wallet_address, &bulk_signers)
        .await?;

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

#[allow(clippy::too_many_arguments)]
async fn command_approve(
    config: &Config<'_>,
    account: Pubkey,
    owner: Pubkey,
    ui_amount: f64,
    delegate: Pubkey,
    mint_address: Option<Pubkey>,
    mint_decimals: Option<u8>,
    use_unchecked_instruction: bool,
    bulk_signers: BulkSigners,
) -> CommandResult {
    println_display(
        config,
        format!(
            "Approve {} tokens\n  Account: {}\n  Delegate: {}",
            ui_amount, account, delegate
        ),
    );

    let mint_address = config.check_account(&account, mint_address).await?;
    let mint_info = config.get_mint_info(&mint_address, mint_decimals).await?;
    let amount = spl_token::ui_amount_to_amount(ui_amount, mint_info.decimals);
    let decimals = if use_unchecked_instruction {
        None
    } else {
        Some(mint_info.decimals)
    };

    let token = token_client_from_config(config, &mint_info.address);
    let res = token
        .approve(&account, &delegate, &owner, amount, decimals, &bulk_signers)
        .await?;

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

async fn command_revoke(
    config: &Config<'_>,
    account: Pubkey,
    owner: Pubkey,
    delegate: Option<Pubkey>,
    bulk_signers: BulkSigners,
) -> CommandResult {
    let (mint_pubkey, delegate) = if !config.sign_only {
        let source_account = config.get_account_checked(&account).await?;
        let source_state = StateWithExtensionsOwned::<Account>::unpack(source_account.data)
            .map_err(|_| format!("Could not deserialize token account {}", account))?;

        let delegate = if let COption::Some(delegate) = source_state.base.delegate {
            Some(delegate)
        } else {
            None
        };

        (source_state.base.mint, delegate)
    } else {
        // default is safe here because revoke doesnt use it
        (Pubkey::default(), delegate)
    };

    if let Some(delegate) = delegate {
        println_display(
            config,
            format!(
                "Revoking approval\n  Account: {}\n  Delegate: {}",
                account, delegate
            ),
        );
    } else {
        return Err(format!("No delegate on account {}", account).into());
    }

    let token = token_client_from_config(config, &mint_pubkey);
    let res = token.revoke(&account, &owner, &bulk_signers).await?;

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

async fn command_close(
    config: &Config<'_>,
    account: Pubkey,
    close_authority: Pubkey,
    recipient: Pubkey,
    bulk_signers: BulkSigners,
) -> CommandResult {
    let mint_pubkey = if !config.sign_only {
        let source_account = config.get_account_checked(&account).await?;

        let source_state = StateWithExtensionsOwned::<Account>::unpack(source_account.data)
            .map_err(|_| format!("Could not deserialize token account {}", account))?;
        let source_amount = source_state.base.amount;

        if !source_state.base.is_native() && source_amount > 0 {
            return Err(format!(
                "Account {} still has {} tokens; empty the account in order to close it.",
                account, source_amount,
            )
            .into());
        }

        source_state.base.mint
    } else {
        // default is safe here because close doesnt use it
        Pubkey::default()
    };

    let token = token_client_from_config(config, &mint_pubkey);
    let res = token
        .close_account(&account, &recipient, &close_authority, &bulk_signers)
        .await?;

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

async fn command_close_mint(
    config: &Config<'_>,
    token_pubkey: Pubkey,
    close_authority: Pubkey,
    recipient: Pubkey,
    bulk_signers: BulkSigners,
) -> CommandResult {
    if !config.sign_only {
        let mint_account = config.get_account_checked(&token_pubkey).await?;

        let mint_state = StateWithExtensionsOwned::<Mint>::unpack(mint_account.data)
            .map_err(|_| format!("Could not deserialize token mint {}", token_pubkey))?;
        let mint_supply = mint_state.base.supply;

        if mint_supply > 0 {
            return Err(format!(
                "Mint {} still has {} outstanding tokens; these must be burned before closing the mint.",
                token_pubkey, mint_supply,
            )
            .into());
        }

        if let Ok(mint_close_authority) = mint_state.get_extension::<MintCloseAuthority>() {
            let mint_close_authority_pubkey =
                Option::<Pubkey>::from(mint_close_authority.close_authority);

            if mint_close_authority_pubkey != Some(close_authority) {
                return Err(format!(
                    "Mint {} has close authority {}, but {} was provided",
                    token_pubkey,
                    mint_close_authority_pubkey
                        .map(|pubkey| pubkey.to_string())
                        .unwrap_or_else(|| "disabled".to_string()),
                    close_authority
                )
                .into());
            }
        } else {
            return Err(format!("Mint {} does not support close authority", token_pubkey).into());
        }
    }

    let token = token_client_from_config(config, &token_pubkey);
    let res = token
        .close_account(&token_pubkey, &recipient, &close_authority, &bulk_signers)
        .await?;

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

async fn command_balance(config: &Config<'_>, address: Pubkey) -> CommandResult {
    let balance = config
        .rpc_client
        .get_token_account_balance(&address)
        .await
        .map_err(|_| format!("Could not find token account {}", address))?;
    let cli_token_amount = CliTokenAmount { amount: balance };
    Ok(config.output_format.formatted_string(&cli_token_amount))
}

async fn command_supply(config: &Config<'_>, token: Pubkey) -> CommandResult {
    let supply = config.rpc_client.get_token_supply(&token).await?;
    let cli_token_amount = CliTokenAmount { amount: supply };
    Ok(config.output_format.formatted_string(&cli_token_amount))
}

async fn command_accounts(
    config: &Config<'_>,
    token: Option<Pubkey>,
    owner: Pubkey,
) -> CommandResult {
    if let Some(token) = token {
        validate_mint(config, token).await?;
    }
    let accounts = config
        .rpc_client
        .get_token_accounts_by_owner(
            &owner,
            match token {
                Some(token) => TokenAccountsFilter::Mint(token),
                None => TokenAccountsFilter::ProgramId(config.program_id),
            },
        )
        .await?;
    if accounts.is_empty() {
        println!("None");
        return Ok("".to_string());
    }

    let (mint_accounts, unsupported_accounts, max_len_balance, includes_aux) =
        sort_and_parse_token_accounts(&owner, accounts, &config.program_id);
    let aux_len = if includes_aux { 10 } else { 0 };

    let cli_token_accounts = CliTokenAccounts {
        accounts: mint_accounts
            .into_iter()
            .map(|(_mint, accounts_list)| accounts_list)
            .collect(),
        unsupported_accounts,
        max_len_balance,
        aux_len,
        token_is_some: token.is_some(),
    };
    Ok(config.output_format.formatted_string(&cli_token_accounts))
}

async fn command_address(
    config: &Config<'_>,
    token: Option<Pubkey>,
    owner: Pubkey,
) -> CommandResult {
    let mut cli_address = CliWalletAddress {
        wallet_address: owner.to_string(),
        ..CliWalletAddress::default()
    };
    if let Some(token) = token {
        validate_mint(config, token).await?;
        let associated_token_address =
            get_associated_token_address_with_program_id(&owner, &token, &config.program_id);
        cli_address.associated_token_address = Some(associated_token_address.to_string());
    }
    Ok(config.output_format.formatted_string(&cli_address))
}

async fn command_display(config: &Config<'_>, address: Pubkey) -> CommandResult {
    let account_data = config.get_account_checked(&address).await?;

    let decimals = if let Some(mint_address) = get_token_account_mint(&account_data.data) {
        Some(config.get_mint_info(&mint_address, None).await?.decimals)
    } else {
        None
    };

    let token_data = parse_token(&account_data.data, decimals);

    match token_data {
        Ok(TokenAccountType::Account(account)) => {
            let mint_address = Pubkey::from_str(&account.mint)?;
            let owner = Pubkey::from_str(&account.owner)?;
            let associated_address = get_associated_token_address_with_program_id(
                &owner,
                &mint_address,
                &config.program_id,
            );

            let cli_output = CliTokenAccount {
                address: address.to_string(),
                program_id: config.program_id.to_string(),
                decimals,
                is_associated: associated_address == address,
                account,
            };

            Ok(config.output_format.formatted_string(&cli_output))
        }
        Ok(TokenAccountType::Mint(mint)) => {
            let epoch_info = config.rpc_client.get_epoch_info().await?;
            let cli_output = CliMint {
                address: address.to_string(),
                epoch: epoch_info.epoch,
                program_id: config.program_id.to_string(),
                mint,
            };

            Ok(config.output_format.formatted_string(&cli_output))
        }
        Ok(TokenAccountType::Multisig(multisig)) => {
            let cli_output = CliMultisig {
                address: address.to_string(),
                program_id: config.program_id.to_string(),
                multisig,
            };

            Ok(config.output_format.formatted_string(&cli_output))
        }
        Err(e) => Err(e.into()),
    }
}

async fn command_gc(
    config: &Config<'_>,
    owner: Pubkey,
    close_empty_associated_accounts: bool,
    bulk_signers: BulkSigners,
) -> CommandResult {
    println_display(
        config,
        format!(
            "Fetching token accounts associated with program {}",
            config.program_id
        ),
    );
    let accounts = config
        .rpc_client
        .get_token_accounts_by_owner(&owner, TokenAccountsFilter::ProgramId(config.program_id))
        .await?;
    if accounts.is_empty() {
        println_display(config, "Nothing to do".to_string());
        return Ok("".to_string());
    }

    let minimum_balance_for_rent_exemption = if !config.sign_only {
        config
            .program_client
            .get_minimum_balance_for_rent_exemption(Account::LEN)
            .await?
    } else {
        0
    };

    let mut accounts_by_token = HashMap::new();

    for keyed_account in accounts {
        if let UiAccountData::Json(parsed_account) = keyed_account.account.data {
            if is_supported_program(&parsed_account.program) {
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
                    let token_amount = ui_token_account
                        .token_amount
                        .amount
                        .parse::<u64>()
                        .unwrap_or_else(|err| panic!("Invalid token amount: {}", err));

                    let close_authority = ui_token_account.close_authority.map_or(owner, |s| {
                        s.parse::<Pubkey>()
                            .unwrap_or_else(|err| panic!("Invalid close authority: {}", err))
                    });

                    let entry = accounts_by_token.entry(token).or_insert_with(HashMap::new);
                    entry.insert(
                        token_account,
                        (
                            token_amount,
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
        println_display(config, format!("Processing token: {}", token));
        let associated_token_account =
            get_associated_token_address_with_program_id(&owner, &token, &config.program_id);
        let total_balance: u64 = accounts.values().map(|account| account.0).sum();

        if total_balance > 0 && !accounts.contains_key(&associated_token_account) {
            // Create the associated token account
            instructions.push(vec![create_associated_token_account(
                &config.fee_payer.pubkey(),
                &owner,
                &token,
                &config.program_id,
            )]);
            lamports_needed += minimum_balance_for_rent_exemption;
        }

        for (address, (amount, decimals, frozen, close_authority)) in accounts {
            match (
                address == associated_token_account,
                close_empty_associated_accounts,
                total_balance > 0,
            ) {
                (true, _, true) => continue, // don't ever close associated token account with amount
                (true, false, _) => continue, // don't close associated token account if close_empty_associated_accounts isn't set
                (true, true, false) => println_display(
                    config,
                    format!("Closing Account {}", associated_token_account),
                ),
                _ => {}
            }

            if frozen {
                // leave frozen accounts alone
                continue;
            }

            let mut account_instructions = vec![];

            // Sanity check!
            // we shouldn't ever be here, but if we are here, abort!
            assert!(amount == 0 || address != associated_token_account);

            if amount > 0 {
                // Transfer the account balance into the associated token account
                account_instructions.push(transfer_checked(
                    &config.program_id,
                    &address,
                    &token,
                    &associated_token_account,
                    &owner,
                    &config.multisigner_pubkeys,
                    amount,
                    decimals,
                )?);
            }
            // Close the account if config.owner is able to
            if close_authority == owner {
                account_instructions.push(close_account(
                    &config.program_id,
                    &address,
                    &owner,
                    &owner,
                    &config.multisigner_pubkeys,
                )?);
            }

            if !account_instructions.is_empty() {
                instructions.push(account_instructions);
            }
        }
    }

    let cli_signer_info = CliSignerInfo {
        signers: bulk_signers,
    };

    let mut result = String::from("");
    for tx_instructions in instructions {
        let tx_return = handle_tx(
            &cli_signer_info,
            config,
            false,
            lamports_needed,
            tx_instructions,
        )
        .await?;
        result += &match tx_return {
            TransactionReturnData::CliSignature(signature) => {
                config.output_format.formatted_string(&signature)
            }
            TransactionReturnData::CliSignOnlyData(sign_only_data) => {
                config.output_format.formatted_string(&sign_only_data)
            }
        };
        result += "\n";
    }
    Ok(result)
}

async fn command_sync_native(config: &Config<'_>, native_account_address: Pubkey) -> CommandResult {
    let token = native_token_client_from_config(config);

    if !config.sign_only {
        let account_data = config.get_account_checked(&native_account_address).await?;
        let account_state = StateWithExtensionsOwned::<Account>::unpack(account_data.data)?;

        if account_state.base.mint != *token.get_address() {
            return Err(format!("{} is not a native token account", native_account_address).into());
        }
    }

    let res = token.sync_native(&native_account_address).await?;
    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
}

// Both enable_required_transfer_mesos and disable_required_transfer_mesos
// Switches with enable_memos bool
async fn command_required_transfer_memos(
    config: &Config<'_>,
    token_account_address: Pubkey,
    owner: Pubkey,
    bulk_signers: BulkSigners,
    enable_memos: bool,
) -> CommandResult {
    if config.sign_only {
        panic!("Config can not be sign only for enabling/disabling required transfer memos.");
    }

    let account = config.get_account_checked(&token_account_address).await?;
    let current_account_len = account.data.len();

    let state_with_extension = StateWithExtensionsOwned::<Account>::unpack(account.data)?;
    let token = token_client_from_config(config, &state_with_extension.base.mint);

    // Reallocation (if needed)
    let mut existing_extensions: Vec<ExtensionType> = state_with_extension.get_extension_types()?;
    if existing_extensions.contains(&ExtensionType::MemoTransfer) {
        let extension_data: bool = state_with_extension
            .get_extension::<MemoTransfer>()?
            .require_incoming_transfer_memos
            .into();
        if extension_data == enable_memos {
            return Ok(format!(
                "Required memo transfer was already {}",
                if extension_data {
                    "enabled"
                } else {
                    "disabled"
                }
            ));
        }
    } else {
        existing_extensions.push(ExtensionType::MemoTransfer);
        let needed_account_len = ExtensionType::get_account_len::<Account>(&existing_extensions);
        if needed_account_len > current_account_len {
            token
                .reallocate(
                    &token_account_address,
                    &owner,
                    &[ExtensionType::MemoTransfer],
                    &bulk_signers,
                )
                .await?;
        }
    }

    let res = if enable_memos {
        token
            .enable_required_transfer_memos(&token_account_address, &owner, &bulk_signers)
            .await
    } else {
        token
            .disable_required_transfer_memos(&token_account_address, &owner, &bulk_signers)
            .await
    }?;

    let tx_return = finish_tx(config, &res, false).await?;
    Ok(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    })
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

fn minimum_signers_help_string() -> String {
    format!(
        "The minimum number of signers required to allow the operation. [{} <= M <= N]",
        MIN_SIGNERS
    )
}

fn multisig_member_help_string() -> String {
    format!(
        "The public keys for each of the N signing members of this account. [{} <= N <= {}]",
        MIN_SIGNERS, MAX_SIGNERS
    )
}

fn is_valid_token_program_id<T>(string: T) -> Result<(), String>
where
    T: AsRef<str> + Display,
{
    match is_pubkey(string.as_ref()) {
        Ok(()) => {
            let program_id = string.as_ref().parse::<Pubkey>().unwrap();
            if program_id == spl_token_2022::id() || program_id == spl_token::id() {
                Ok(())
            } else {
                Err(format!("Unrecognized token program id: {}", program_id))
            }
        }
        Err(e) => Err(e),
    }
}

fn app<'a, 'b>(
    default_decimals: &'a str,
    minimum_signers_help: &'b str,
    multisig_member_help: &'b str,
) -> App<'a, 'b> {
    App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::with_name("config_file")
                .short("C")
                .long("config")
                .value_name("PATH")
                .takes_value(true)
                .global(true)
                .help("Configuration file to use"),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .takes_value(false)
                .global(true)
                .help("Show additional information"),
        )
        .arg(
            Arg::with_name("output_format")
                .long("output")
                .value_name("FORMAT")
                .global(true)
                .takes_value(true)
                .possible_values(&["json", "json-compact"])
                .help("Return information in specified output format"),
        )
        .arg(
            Arg::with_name("program_id")
                .short("p")
                .long("program-id")
                .value_name("ADDRESS")
                .takes_value(true)
                .global(true)
                .validator(is_valid_token_program_id)
                .help("SPL Token program id"),
        )
        .arg(
            Arg::with_name("json_rpc_url")
                .short("u")
                .long("url")
                .value_name("URL_OR_MONIKER")
                .takes_value(true)
                .global(true)
                .validator(is_url_or_moniker)
                .help(
                    "URL for Solana's JSON RPC or moniker (or their first letter): \
                       [mainnet-beta, testnet, devnet, localhost] \
                    Default from the configuration file."
                ),
        )
        .arg(fee_payer_arg().global(true))
        .arg(
            Arg::with_name("use_unchecked_instruction")
                .long("use-unchecked-instruction")
                .takes_value(false)
                .global(true)
                .hidden(true)
                .help("Use unchecked instruction if appropriate. Supports transfer, burn, mint, and approve."),
        )
        .bench_subcommand()
        .subcommand(SubCommand::with_name(CommandName::CreateToken.into()).about("Create a new token")
                .arg(
                    Arg::with_name("token_keypair")
                        .value_name("TOKEN_KEYPAIR")
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
                    Arg::with_name("mint_authority")
                        .long("mint-authority")
                        .alias("owner")
                        .value_name("ADDRESS")
                        .validator(is_valid_pubkey)
                        .takes_value(true)
                        .help(
                            "Specify the mint authority address. \
                             Defaults to the client keypair address."
                        ),
                )
                .arg(
                    Arg::with_name("decimals")
                        .long("decimals")
                        .validator(is_mint_decimals)
                        .value_name("DECIMALS")
                        .takes_value(true)
                        .default_value(default_decimals)
                        .help("Number of base 10 digits to the right of the decimal place"),
                )
                .arg(
                    Arg::with_name("enable_freeze")
                        .long("enable-freeze")
                        .takes_value(false)
                        .help(
                            "Enable the mint authority to freeze token accounts for this mint"
                        ),
                )
                .arg(
                    Arg::with_name("enable_close")
                        .long("enable-close")
                        .takes_value(false)
                        .help(
                            "Enable the mint authority to close this mint"
                        ),
                )
                .arg(
                    Arg::with_name("interest_rate")
                        .long("interest-rate")
                        .value_name("RATE_BPS")
                        .takes_value(true)
                        .help(
                            "Specify the interest rate in basis points. \
                            Rate authority defaults to the mint authority."
                        ),
                )
                .nonce_args(true)
                .arg(memo_arg())
        )
        .subcommand(
            SubCommand::with_name(CommandName::SetInterestRate.into())
                .about("Set the interest rate for an interest-bearing token")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("The interest-bearing token address"),
                )
                .arg(
                    Arg::with_name("rate")
                        .value_name("RATE")
                        .takes_value(true)
                        .required(true)
                        .help("The new interest rate in basis points"),
                )
                .arg(
                    Arg::with_name("rate_authority")
                    .long("rate-authority")
                    .validator(is_valid_signer)
                    .value_name("SIGNER")
                    .takes_value(true)
                    .help(
                        "Specify the rate authority keypair. \
                        Defaults to the client keypair address."
                    )
                )
        )
        .subcommand(
            SubCommand::with_name(CommandName::CreateAccount.into())
                .about("Create a new token account")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token that the account will hold"),
                )
                .arg(
                    Arg::with_name("account_keypair")
                        .value_name("ACCOUNT_KEYPAIR")
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .index(2)
                        .help(
                            "Specify the account keypair. \
                             This may be a keypair file or the ASK keyword. \
                             [default: associated token account for --owner]"
                        ),
                )
                .arg(
                    Arg::with_name("immutable")
                        .long("immutable")
                        .takes_value(false)
                        .help(
                            "Lock the owner of this token account from ever being changed"
                        ),
                )
                .arg(owner_address_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::CreateMultisig.into())
                .about("Create a new account describing an M:N multisignature")
                .arg(
                    Arg::with_name("minimum_signers")
                        .value_name("MINIMUM_SIGNERS")
                        .validator(is_multisig_minimum_signers)
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help(minimum_signers_help),
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
                        .help(multisig_member_help),
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
        )
        .subcommand(
            SubCommand::with_name(CommandName::Authorize.into())
                .about("Authorize a new signing keypair to a token or token account")
                .arg(
                    Arg::with_name("address")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token mint or account"),
                )
                .arg(
                    Arg::with_name("authority_type")
                        .value_name("AUTHORITY_TYPE")
                        .takes_value(true)
                        .possible_values(&[
                            "mint", "freeze", "owner", "close",
                            "close-mint", "transfer-fee-config", "withheld-withdraw",
                            "interest-rate",
                        ])
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
                    Arg::with_name("authority")
                        .long("authority")
                        .alias("owner")
                        .value_name("KEYPAIR")
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help(
                            "Specify the current authority keypair. \
                             Defaults to the client keypair."
                        ),
                )
                .arg(
                    Arg::with_name("disable")
                        .long("disable")
                        .takes_value(false)
                        .conflicts_with("new_authority")
                        .help("Disable mint, freeze, or close functionality by setting authority to None.")
                )
                .arg(
                    Arg::with_name("force")
                        .long("force")
                        .hidden(true)
                        .help("Force re-authorize the wallet's associate token account. Don't use this flag"),
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Transfer.into())
                .about("Transfer tokens between accounts")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("Token to transfer"),
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
                    Arg::with_name("from")
                        .validator(is_valid_pubkey)
                        .value_name("SENDER_TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .long("from")
                        .help("Specify the sending token account \
                            [default: owner's associated token account]")
                )
                .arg(owner_keypair_arg_with_value_name("SENDER_TOKEN_OWNER_KEYPAIR")
                        .help(
                            "Specify the owner of the sending token account. \
                            This may be a keypair file or the ASK keyword. \
                            Defaults to the client keypair.",
                        ),
                )
                .arg(
                    Arg::with_name("allow_unfunded_recipient")
                        .long("allow-unfunded-recipient")
                        .takes_value(false)
                        .help("Complete the transfer even if the recipient address is not funded")
                )
                .arg(
                    Arg::with_name("allow_empty_recipient")
                        .long("allow-empty-recipient")
                        .takes_value(false)
                        .hidden(true) // Deprecated, use --allow-unfunded-recipient instead
                )
                .arg(
                    Arg::with_name("fund_recipient")
                        .long("fund-recipient")
                        .takes_value(false)
                        .help("Create the associated token account for the recipient if doesn't already exist")
                )
                .arg(
                    Arg::with_name("no_wait")
                        .long("no-wait")
                        .takes_value(false)
                        .help("Return signature immediately after submitting the transaction, instead of waiting for confirmations"),
                )
                .arg(
                    Arg::with_name("allow_non_system_account_recipient")
                        .long("allow-non-system-account-recipient")
                        .takes_value(false)
                        .help("Send tokens to the recipient even if the recipient is not a wallet owned by System Program."),
                )
                .arg(
                    Arg::with_name("recipient_is_ata_owner")
                        .long("recipient-is-ata-owner")
                        .takes_value(false)
                        .requires("sign_only")
                        .help("In sign-only mode, specifies that the recipient is the owner of the associated token account rather than an actual token account"),
                )
                .arg(multisig_signer_arg())
                .arg(mint_decimals_arg())
                .nonce_args(true)
                .arg(memo_arg())
                .offline_args_config(&SignOnlyNeedsMintDecimals{}),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Burn.into())
                .about("Burn tokens from an account")
                .arg(
                    Arg::with_name("account")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
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
                .arg(owner_keypair_arg_with_value_name("TOKEN_OWNER_KEYPAIR")
                        .help(
                            "Specify the burnt token owner account. \
                            This may be a keypair file or the ASK keyword. \
                            Defaults to the client keypair.",
                        ),
                )
                .arg(multisig_signer_arg())
                .mint_args()
                .nonce_args(true)
                .arg(memo_arg())
                .offline_args_config(&SignOnlyNeedsFullMintSpec{}),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Mint.into())
                .about("Mint new tokens")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_MINT_ADDRESS")
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
                .arg(
                    Arg::with_name("mint_authority")
                        .long("mint-authority")
                        .alias("owner")
                        .value_name("KEYPAIR")
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help(
                            "Specify the mint authority keypair. \
                             This may be a keypair file or the ASK keyword. \
                             Defaults to the client keypair."
                        ),
                )
                .arg(mint_decimals_arg())
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .arg(memo_arg())
                .offline_args_config(&SignOnlyNeedsMintDecimals{}),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Freeze.into())
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
                .arg(
                    Arg::with_name("freeze_authority")
                        .long("freeze-authority")
                        .alias("owner")
                        .value_name("KEYPAIR")
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help(
                            "Specify the freeze authority keypair. \
                             This may be a keypair file or the ASK keyword. \
                             Defaults to the client keypair."
                        ),
                )
                .arg(mint_address_arg())
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args_config(&SignOnlyNeedsMintAddress{}),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Thaw.into())
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
                .arg(
                    Arg::with_name("freeze_authority")
                        .long("freeze-authority")
                        .alias("owner")
                        .value_name("KEYPAIR")
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help(
                            "Specify the freeze authority keypair. \
                             This may be a keypair file or the ASK keyword. \
                             Defaults to the client keypair."
                        ),
                )
                .arg(mint_address_arg())
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args_config(&SignOnlyNeedsMintAddress{}),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Wrap.into())
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
                .arg(
                    Arg::with_name("wallet_keypair")
                        .alias("owner")
                        .value_name("KEYPAIR")
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help(
                            "Specify the keypair for the wallet which will have its native SOL wrapped. \
                             This wallet will be assigned as the owner of the wrapped SOL token account. \
                             This may be a keypair file or the ASK keyword. \
                             Defaults to the client keypair."
                        ),
                )
                .arg(
                    Arg::with_name("create_aux_account")
                        .takes_value(false)
                        .long("create-aux-account")
                        .help("Wrap SOL in an auxiliary account instead of associated token account"),
                )
                .arg(
                    Arg::with_name("immutable")
                        .long("immutable")
                        .takes_value(false)
                        .help(
                            "Lock the owner of this token account from ever being changed"
                        ),
                )
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Unwrap.into())
                .about("Unwrap a SOL token account")
                .arg(
                    Arg::with_name("account")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .help("The address of the auxiliary token account to unwrap \
                            [default: associated token account for --owner]"),
                )
                .arg(
                    Arg::with_name("wallet_keypair")
                        .alias("owner")
                        .value_name("KEYPAIR")
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help(
                            "Specify the keypair for the wallet which owns the wrapped SOL. \
                             This wallet will receive the unwrapped SOL. \
                             This may be a keypair file or the ASK keyword. \
                             Defaults to the client keypair."
                        ),
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Approve.into())
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
                .arg(
                    owner_keypair_arg()
                )
                .arg(multisig_signer_arg())
                .mint_args()
                .nonce_args(true)
                .offline_args_config(&SignOnlyNeedsFullMintSpec{}),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Revoke.into())
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
                .arg(owner_keypair_arg()
                )
                .arg(delegate_address_arg())
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args_config(&SignOnlyNeedsDelegateAddress{}),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Close.into())
                .about("Close a token account")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required_unless("address")
                        .help("Token to close. To close a specific account, use the `--address` parameter instead"),
                )
                .arg(owner_address_arg())
                .arg(
                    Arg::with_name("recipient")
                        .long("recipient")
                        .validator(is_valid_pubkey)
                        .value_name("REFUND_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .help("The address of the account to receive remaining SOL [default: --owner]"),
                )
                .arg(
                    Arg::with_name("close_authority")
                        .long("close-authority")
                        .alias("owner")
                        .value_name("KEYPAIR")
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help(
                            "Specify the token's close authority if it has one, \
                            otherwise specify the token's owner keypair. \
                            This may be a keypair file or the ASK keyword. \
                            Defaults to the client keypair.",
                        ),
                )
                .arg(
                    Arg::with_name("address")
                        .long("address")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .conflicts_with("token")
                        .help("Specify the token account to close \
                            [default: owner's associated token account]"),
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name(CommandName::CloseMint.into())
                .about("Close a token mint")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("Token to close"),
                )
                .arg(owner_address_arg())
                .arg(
                    Arg::with_name("recipient")
                        .long("recipient")
                        .validator(is_valid_pubkey)
                        .value_name("REFUND_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .help("The address of the account to receive remaining SOL [default: --owner]"),
                )
                .arg(
                    Arg::with_name("close_authority")
                        .long("close-authority")
                        .value_name("KEYPAIR")
                        .validator(is_valid_signer)
                        .takes_value(true)
                        .help(
                            "Specify the token's close authority. \
                            This may be a keypair file or the ASK keyword. \
                            Defaults to the client keypair.",
                        ),
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Balance.into())
                .about("Get token account balance")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required_unless("address")
                        .help("Token of associated account. To query a specific account, use the `--address` parameter instead"),
                )
                .arg(owner_address_arg().conflicts_with("address"))
                .arg(
                    Arg::with_name("address")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .long("address")
                        .conflicts_with("token")
                        .help("Specify the token account to query \
                            [default: owner's associated token account]"),
                ),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Supply.into())
                .about("Get token supply")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token address"),
                ),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Accounts.into())
                .about("List all token accounts by owner")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .help("Limit results to the given token. [Default: list accounts for all tokens]"),
                )
                .arg(owner_address_arg())
        )
        .subcommand(
            SubCommand::with_name(CommandName::Address.into())
                .about("Get wallet address")
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .long("token")
                        .requires("verbose")
                        .help("Return the associated token address for the given token. \
                               [Default: return the client keypair address]")
                )
                .arg(
                    owner_address_arg()
                        .requires("token")
                        .help("Return the associated token address for the given owner. \
                               [Default: return the associated token address for the client keypair]"),
                ),
        )
        .subcommand(
            SubCommand::with_name(CommandName::AccountInfo.into())
                .about("Query details of an SPL Token account by address (DEPRECATED: use `spl-token display`)")
                .setting(AppSettings::Hidden)
                .arg(
                    Arg::with_name("token")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .conflicts_with("address")
                        .required_unless("address")
                        .help("Token of associated account. \
                               To query a specific account, use the `--address` parameter instead"),
                )
                .arg(
                    owner_address_arg()
                        .index(2)
                        .conflicts_with("address")
                        .help("Owner of the associated account for the specified token. \
                               To query a specific account, use the `--address` parameter instead. \
                               Defaults to the client keypair."),
                )
                .arg(
                    Arg::with_name("address")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .long("address")
                        .conflicts_with("token")
                        .help("Specify the token account to query"),
                ),
        )
        .subcommand(
            SubCommand::with_name(CommandName::MultisigInfo.into())
                .about("Query details of an SPL Token multisig account by address (DEPRECATED: use `spl-token display`)")
                .setting(AppSettings::Hidden)
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
            SubCommand::with_name(CommandName::Display.into())
                .about("Query details of an SPL Token mint, account, or multisig by address")
                .arg(
                    Arg::with_name("address")
                    .validator(is_valid_pubkey)
                    .value_name("TOKEN_ADDRESS")
                    .takes_value(true)
                    .index(1)
                    .required(true)
                    .help("The address of the SPL Token mint, account, or multisig to query"),
                ),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Gc.into())
                .about("Cleanup unnecessary token accounts")
                .arg(owner_keypair_arg())
                .arg(
                    Arg::with_name("close_empty_associated_accounts")
                    .long("close-empty-associated-accounts")
                    .takes_value(false)
                    .help("close all empty associated token accounts (to get SOL back)")
                )
        )
        .subcommand(
            SubCommand::with_name(CommandName::SyncNative.into())
                .about("Sync a native SOL token account to its underlying lamports")
                .arg(
                    owner_address_arg()
                        .index(1)
                        .conflicts_with("address")
                        .help("Owner of the associated account for the native token. \
                               To query a specific account, use the `--address` parameter instead. \
                               Defaults to the client keypair."),
                )
                .arg(
                    Arg::with_name("address")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .long("address")
                        .conflicts_with("owner")
                        .help("Specify the specific token account address to sync"),
                ),
        )
        .subcommand(
            SubCommand::with_name(CommandName::EnableRequiredTransferMemos.into())
                .about("Enable required transfer memos for token account")
                .arg(
                    Arg::with_name("account")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to enable required transfer memos")
                )
                .arg(
                    owner_address_arg()
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::DisableRequiredTransferMemos.into())
                .about("Disable required transfer memos for token account")
                .arg(
                    Arg::with_name("account")
                        .validator(is_valid_pubkey)
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to disable required transfer memos"),
                )
                .arg(
                    owner_address_arg()
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
        )
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let default_decimals = format!("{}", spl_token_2022::native_mint::DECIMALS);
    let minimum_signers_help = minimum_signers_help_string();
    let multisig_member_help = multisig_member_help_string();
    let app_matches = app(
        &default_decimals,
        &minimum_signers_help,
        &multisig_member_help,
    )
    .get_matches();

    let mut wallet_manager = None;
    let mut bulk_signers: Vec<Arc<dyn Signer>> = Vec::new();

    let (sub_command, sub_matches) = app_matches.subcommand();
    let sub_command = CommandName::from_str(sub_command).unwrap();
    let matches = sub_matches.unwrap();

    let mut multisigner_ids = Vec::new();
    let config = Config::new(
        matches,
        &mut wallet_manager,
        &mut bulk_signers,
        &mut multisigner_ids,
    )
    .await;

    solana_logger::setup_with_default("solana=info");
    let result =
        process_command(&sub_command, matches, &config, wallet_manager, bulk_signers).await?;
    println!("{}", result);
    Ok(())
}

async fn process_command<'a>(
    sub_command: &CommandName,
    sub_matches: &ArgMatches<'_>,
    config: &Config<'a>,
    mut wallet_manager: Option<Arc<RemoteWalletManager>>,
    mut bulk_signers: Vec<Arc<dyn Signer>>,
) -> CommandResult {
    match (sub_command, sub_matches) {
        (CommandName::Bench, arg_matches) => {
            bench_process_command(
                arg_matches,
                config,
                std::mem::take(&mut bulk_signers),
                &mut wallet_manager,
            )
            .await
        }
        (CommandName::CreateToken, arg_matches) => {
            let decimals = value_t_or_exit!(arg_matches, "decimals", u8);
            let mint_authority =
                config.pubkey_or_default(arg_matches, "mint_authority", &mut wallet_manager);
            let memo = value_t!(arg_matches, "memo", String).ok();
            let rate_bps = value_t!(arg_matches, "interest_rate", i16).ok();

            let (token_signer, token) =
                get_signer(arg_matches, "token_keypair", &mut wallet_manager)
                    .unwrap_or_else(new_throwaway_signer);
            if !bulk_signers.contains(&token_signer) {
                bulk_signers.push(token_signer);
            }

            command_create_token(
                config,
                decimals,
                token,
                mint_authority,
                arg_matches.is_present("enable_freeze"),
                arg_matches.is_present("enable_close"),
                memo,
                rate_bps,
                bulk_signers,
            )
            .await
        }
        (CommandName::SetInterestRate, arg_matches) => {
            let token_pubkey = pubkey_of_signer(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let rate_bps = value_t_or_exit!(arg_matches, "rate", i16);
            let (rate_authority_signer, rate_authority_pubkey) =
                config.signer_or_default(arg_matches, "rate_authority", &mut wallet_manager);
            let bulk_signers = vec![rate_authority_signer];

            command_set_interest_rate(
                config,
                token_pubkey,
                rate_authority_pubkey,
                rate_bps,
                bulk_signers,
            )
            .await
        }
        (CommandName::CreateAccount, arg_matches) => {
            let token = pubkey_of_signer(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();

            // No need to add a signer when creating an associated token account
            let account = get_signer(arg_matches, "account_keypair", &mut wallet_manager).map(
                |(signer, account)| {
                    if !bulk_signers.contains(&signer) {
                        bulk_signers.push(signer);
                    }
                    account
                },
            );

            let owner = config.pubkey_or_default(arg_matches, "owner", &mut wallet_manager);
            command_create_account(
                config,
                token,
                owner,
                account,
                arg_matches.is_present("immutable"),
                bulk_signers,
            )
            .await
        }
        (CommandName::CreateMultisig, arg_matches) => {
            let minimum_signers = value_of::<u8>(arg_matches, "minimum_signers").unwrap();
            let multisig_members =
                pubkeys_of_multiple_signers(arg_matches, "multisig_member", &mut wallet_manager)
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

            let (signer, _) = get_signer(arg_matches, "address_keypair", &mut wallet_manager)
                .unwrap_or_else(new_throwaway_signer);

            command_create_multisig(config, signer, minimum_signers, multisig_members).await
        }
        (CommandName::Authorize, arg_matches) => {
            let address = pubkey_of_signer(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let authority_type = arg_matches.value_of("authority_type").unwrap();
            let authority_type = match authority_type {
                "mint" => AuthorityType::MintTokens,
                "freeze" => AuthorityType::FreezeAccount,
                "owner" => AuthorityType::AccountOwner,
                "close" => AuthorityType::CloseAccount,
                "close-mint" => AuthorityType::CloseMint,
                "transfer-fee-config" => AuthorityType::TransferFeeConfig,
                "withheld-withdraw" => AuthorityType::WithheldWithdraw,
                "interest-rate" => AuthorityType::InterestRate,
                _ => unreachable!(),
            };

            let (authority_signer, authority) =
                config.signer_or_default(arg_matches, "authority", &mut wallet_manager);
            if !bulk_signers.contains(&authority_signer) {
                bulk_signers.push(authority_signer);
            }

            let new_authority =
                pubkey_of_signer(arg_matches, "new_authority", &mut wallet_manager).unwrap();
            let force_authorize = arg_matches.is_present("force");
            command_authorize(
                config,
                address,
                authority_type,
                authority,
                new_authority,
                force_authorize,
                bulk_signers,
            )
            .await
        }
        (CommandName::Transfer, arg_matches) => {
            let token = pubkey_of_signer(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let amount = match arg_matches.value_of("amount").unwrap() {
                "ALL" => None,
                amount => Some(amount.parse::<f64>().unwrap()),
            };
            let recipient = pubkey_of_signer(arg_matches, "recipient", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let sender = pubkey_of_signer(arg_matches, "from", &mut wallet_manager).unwrap();

            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            if !bulk_signers.contains(&owner_signer) {
                bulk_signers.push(owner_signer);
            }

            let mint_decimals = value_of::<u8>(arg_matches, MINT_DECIMALS_ARG.name);
            let fund_recipient = arg_matches.is_present("fund_recipient");
            let allow_unfunded_recipient = arg_matches.is_present("allow_empty_recipient")
                || arg_matches.is_present("allow_unfunded_recipient");

            let recipient_is_ata_owner = arg_matches.is_present("recipient_is_ata_owner");
            let use_unchecked_instruction = arg_matches.is_present("use_unchecked_instruction");
            let memo = value_t!(arg_matches, "memo", String).ok();

            command_transfer(
                config,
                token,
                amount,
                recipient,
                sender,
                owner,
                allow_unfunded_recipient,
                fund_recipient,
                mint_decimals,
                recipient_is_ata_owner,
                use_unchecked_instruction,
                memo,
                bulk_signers,
                arg_matches.is_present("no_wait"),
                arg_matches.is_present("allow_non_system_account_recipient"),
            )
            .await
        }
        (CommandName::Burn, arg_matches) => {
            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();

            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            if !bulk_signers.contains(&owner_signer) {
                bulk_signers.push(owner_signer);
            }

            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            let mint_address =
                pubkey_of_signer(arg_matches, MINT_ADDRESS_ARG.name, &mut wallet_manager).unwrap();
            let mint_decimals = value_of::<u8>(arg_matches, MINT_DECIMALS_ARG.name);
            let use_unchecked_instruction = arg_matches.is_present("use_unchecked_instruction");
            let memo = value_t!(arg_matches, "memo", String).ok();
            command_burn(
                config,
                account,
                owner,
                amount,
                mint_address,
                mint_decimals,
                use_unchecked_instruction,
                memo,
                bulk_signers,
            )
            .await
        }
        (CommandName::Mint, arg_matches) => {
            let (mint_authority_signer, mint_authority) =
                config.signer_or_default(arg_matches, "mint_authority", &mut wallet_manager);
            if !bulk_signers.contains(&mint_authority_signer) {
                bulk_signers.push(mint_authority_signer);
            }

            let token = pubkey_of_signer(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            let mint_decimals = value_of::<u8>(arg_matches, MINT_DECIMALS_ARG.name);
            let mint_info = config.get_mint_info(&token, mint_decimals).await?;
            let recipient = if let Some(address) =
                pubkey_of_signer(arg_matches, "recipient", &mut wallet_manager).unwrap()
            {
                address
            } else {
                config.associated_token_address_for_token_and_program(
                    &mint_info.address,
                    &mint_info.program_id,
                )
            };
            config.check_account(&recipient, Some(token)).await?;
            let use_unchecked_instruction = arg_matches.is_present("use_unchecked_instruction");
            let memo = value_t!(arg_matches, "memo", String).ok();
            command_mint(
                config,
                token,
                amount,
                recipient,
                mint_info,
                mint_authority,
                use_unchecked_instruction,
                memo,
                bulk_signers,
            )
            .await
        }
        (CommandName::Freeze, arg_matches) => {
            let (freeze_authority_signer, freeze_authority) =
                config.signer_or_default(arg_matches, "freeze_authority", &mut wallet_manager);
            if !bulk_signers.contains(&freeze_authority_signer) {
                bulk_signers.push(freeze_authority_signer);
            }

            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let mint_address =
                pubkey_of_signer(arg_matches, MINT_ADDRESS_ARG.name, &mut wallet_manager).unwrap();
            command_freeze(
                config,
                account,
                mint_address,
                freeze_authority,
                bulk_signers,
            )
            .await
        }
        (CommandName::Thaw, arg_matches) => {
            let (freeze_authority_signer, freeze_authority) =
                config.signer_or_default(arg_matches, "freeze_authority", &mut wallet_manager);
            if !bulk_signers.contains(&freeze_authority_signer) {
                bulk_signers.push(freeze_authority_signer);
            }

            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let mint_address =
                pubkey_of_signer(arg_matches, MINT_ADDRESS_ARG.name, &mut wallet_manager).unwrap();
            command_thaw(
                config,
                account,
                mint_address,
                freeze_authority,
                bulk_signers,
            )
            .await
        }
        (CommandName::Wrap, arg_matches) => {
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            let account = if arg_matches.is_present("create_aux_account") {
                let (signer, account) = new_throwaway_signer();
                if !bulk_signers.contains(&signer) {
                    bulk_signers.push(signer);
                }
                Some(account)
            } else {
                // No need to add a signer when creating an associated token account
                None
            };

            let (wallet_signer, wallet_address) =
                config.signer_or_default(arg_matches, "wallet_keypair", &mut wallet_manager);
            if !bulk_signers.contains(&wallet_signer) {
                bulk_signers.push(wallet_signer);
            }

            command_wrap(
                config,
                amount,
                wallet_address,
                account,
                arg_matches.is_present("immutable"),
                bulk_signers,
            )
            .await
        }
        (CommandName::Unwrap, arg_matches) => {
            let (wallet_signer, wallet_address) =
                config.signer_or_default(arg_matches, "wallet_keypair", &mut wallet_manager);
            if !bulk_signers.contains(&wallet_signer) {
                bulk_signers.push(wallet_signer);
            }

            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager).unwrap();
            command_unwrap(config, wallet_address, account, bulk_signers).await
        }
        (CommandName::Approve, arg_matches) => {
            let (owner_signer, owner_address) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            if !bulk_signers.contains(&owner_signer) {
                bulk_signers.push(owner_signer);
            }

            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let amount = value_t_or_exit!(arg_matches, "amount", f64);
            let delegate = pubkey_of_signer(arg_matches, "delegate", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let mint_address =
                pubkey_of_signer(arg_matches, MINT_ADDRESS_ARG.name, &mut wallet_manager).unwrap();
            let mint_decimals = value_of::<u8>(arg_matches, MINT_DECIMALS_ARG.name);
            let use_unchecked_instruction = arg_matches.is_present("use_unchecked_instruction");
            command_approve(
                config,
                account,
                owner_address,
                amount,
                delegate,
                mint_address,
                mint_decimals,
                use_unchecked_instruction,
                bulk_signers,
            )
            .await
        }
        (CommandName::Revoke, arg_matches) => {
            let (owner_signer, owner_address) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            if !bulk_signers.contains(&owner_signer) {
                bulk_signers.push(owner_signer);
            }

            let account = pubkey_of_signer(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let delegate_address =
                pubkey_of_signer(arg_matches, DELEGATE_ADDRESS_ARG.name, &mut wallet_manager)
                    .unwrap();
            command_revoke(
                config,
                account,
                owner_address,
                delegate_address,
                bulk_signers,
            )
            .await
        }
        (CommandName::Close, arg_matches) => {
            let (close_authority_signer, close_authority) =
                config.signer_or_default(arg_matches, "close_authority", &mut wallet_manager);
            if !bulk_signers.contains(&close_authority_signer) {
                bulk_signers.push(close_authority_signer);
            }

            let address = config
                .associated_token_address_or_override(arg_matches, "address", &mut wallet_manager)
                .await;
            let recipient = config.pubkey_or_default(arg_matches, "recipient", &mut wallet_manager);
            command_close(config, address, close_authority, recipient, bulk_signers).await
        }
        (CommandName::CloseMint, arg_matches) => {
            let token = pubkey_of_signer(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let (close_authority_signer, close_authority) =
                config.signer_or_default(arg_matches, "close_authority", &mut wallet_manager);
            if !bulk_signers.contains(&close_authority_signer) {
                bulk_signers.push(close_authority_signer);
            }
            let recipient = config.pubkey_or_default(arg_matches, "recipient", &mut wallet_manager);

            command_close_mint(config, token, close_authority, recipient, bulk_signers).await
        }
        (CommandName::Balance, arg_matches) => {
            let address = config
                .associated_token_address_or_override(arg_matches, "address", &mut wallet_manager)
                .await;
            command_balance(config, address).await
        }
        (CommandName::Supply, arg_matches) => {
            let token = pubkey_of_signer(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_supply(config, token).await
        }
        (CommandName::Accounts, arg_matches) => {
            let token = pubkey_of_signer(arg_matches, "token", &mut wallet_manager).unwrap();
            let owner = config.pubkey_or_default(arg_matches, "owner", &mut wallet_manager);
            command_accounts(config, token, owner).await
        }
        (CommandName::Address, arg_matches) => {
            let token = pubkey_of_signer(arg_matches, "token", &mut wallet_manager).unwrap();
            let owner = config.pubkey_or_default(arg_matches, "owner", &mut wallet_manager);
            command_address(config, token, owner).await
        }
        (CommandName::AccountInfo, arg_matches) => {
            let address = config
                .associated_token_address_or_override(arg_matches, "address", &mut wallet_manager)
                .await;
            command_display(config, address).await
        }
        (CommandName::MultisigInfo, arg_matches) => {
            let address = pubkey_of_signer(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_display(config, address).await
        }
        (CommandName::Display, arg_matches) => {
            let address = pubkey_of_signer(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_display(config, address).await
        }
        (CommandName::Gc, arg_matches) => {
            match config.output_format {
                OutputFormat::Json | OutputFormat::JsonCompact => {
                    eprintln!(
                        "`spl-token gc` does not support the `--ouput` parameter at this time"
                    );
                    exit(1);
                }
                _ => {}
            }

            let close_empty_associated_accounts =
                arg_matches.is_present("close_empty_associated_accounts");

            let (owner_signer, owner_address) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            if !bulk_signers.contains(&owner_signer) {
                bulk_signers.push(owner_signer);
            }

            command_gc(
                config,
                owner_address,
                close_empty_associated_accounts,
                bulk_signers,
            )
            .await
        }
        (CommandName::SyncNative, arg_matches) => {
            let native_mint = *native_token_client_from_config(config).get_address();
            let address = config
                .associated_token_address_for_token_or_override(
                    arg_matches,
                    "address",
                    &mut wallet_manager,
                    Some(native_mint),
                )
                .await;
            command_sync_native(config, address).await
        }
        (CommandName::EnableRequiredTransferMemos, arg_matches) => {
            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            if !bulk_signers.contains(&owner_signer) {
                bulk_signers.push(owner_signer);
            }
            // Since account is required argument it will always be present
            let token_account =
                config.pubkey_or_default(arg_matches, "account", &mut wallet_manager);
            command_required_transfer_memos(config, token_account, owner, bulk_signers, true).await
        }
        (CommandName::DisableRequiredTransferMemos, arg_matches) => {
            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            if !bulk_signers.contains(&owner_signer) {
                bulk_signers.push(owner_signer);
            }
            // Since account is required argument it will always be present
            let token_account =
                config.pubkey_or_default(arg_matches, "account", &mut wallet_manager);
            command_required_transfer_memos(config, token_account, owner, bulk_signers, false).await
        }
    }
}

fn format_output<T>(command_output: T, command_name: &CommandName, config: &Config) -> String
where
    T: Serialize + Display + QuietDisplay + VerboseDisplay,
{
    config.output_format.formatted_string(&CommandOutput {
        command_name: command_name.to_string(),
        command_output,
    })
}
enum TransactionReturnData {
    CliSignature(CliSignature),
    CliSignOnlyData(CliSignOnlyData),
}

// XXX this goes away once everything is converted to token client
async fn handle_tx<'a>(
    signer_info: &CliSignerInfo,
    config: &Config<'a>,
    no_wait: bool,
    minimum_balance_for_rent_exemption: u64,
    instructions: Vec<Instruction>,
) -> Result<TransactionReturnData, Error> {
    let fee_payer_pubkey = config.fee_payer.pubkey();
    let fee_payer = Some(&fee_payer_pubkey);

    let recent_blockhash = config.program_client.get_latest_blockhash().await?;
    let message = if let Some(nonce_account) = config.nonce_account.as_ref() {
        let mut message = Message::new_with_nonce(
            instructions,
            fee_payer,
            nonce_account,
            config.nonce_authority.as_ref().unwrap(),
        );
        message.recent_blockhash = recent_blockhash;
        message
    } else {
        Message::new_with_blockhash(&instructions, fee_payer, &recent_blockhash)
    };

    if !config.sign_only {
        let fee = config.rpc_client.get_fee_for_message(&message).await?;
        check_fee_payer_balance(config, minimum_balance_for_rent_exemption + fee).await?;
    }

    let signers = signer_info.signers_for_message(&message);
    let mut transaction = Transaction::new_unsigned(message);

    if config.sign_only {
        transaction.try_partial_sign(&signers, recent_blockhash)?;
        Ok(TransactionReturnData::CliSignOnlyData(return_signers_data(
            &transaction,
            &ReturnSignersConfig {
                dump_transaction_message: config.dump_transaction_message,
            },
        )))
    } else {
        transaction.try_sign(&signers, recent_blockhash)?;
        let signature = if no_wait {
            config.rpc_client.send_transaction(&transaction).await?
        } else {
            config
                .rpc_client
                .send_and_confirm_transaction_with_spinner(&transaction)
                .await?
        };
        Ok(TransactionReturnData::CliSignature(CliSignature {
            signature: signature.to_string(),
        }))
    }
}

async fn finish_tx<'a>(
    config: &Config<'a>,
    rpc_response: &RpcClientResponse,
    no_wait: bool,
) -> Result<TransactionReturnData, Error> {
    match rpc_response {
        RpcClientResponse::Transaction(transaction) => {
            Ok(TransactionReturnData::CliSignOnlyData(return_signers_data(
                transaction,
                &ReturnSignersConfig {
                    dump_transaction_message: config.dump_transaction_message,
                },
            )))
        }
        RpcClientResponse::Signature(signature) if no_wait => {
            Ok(TransactionReturnData::CliSignature(CliSignature {
                signature: signature.to_string(),
            }))
        }
        RpcClientResponse::Signature(signature) => {
            let blockhash = config.program_client.get_latest_blockhash().await?;
            config
                .rpc_client
                .confirm_transaction_with_spinner(
                    signature,
                    &blockhash,
                    config.rpc_client.commitment(),
                )
                .await?;

            Ok(TransactionReturnData::CliSignature(CliSignature {
                signature: signature.to_string(),
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        serial_test::serial,
        solana_sdk::{
            bpf_loader,
            signature::{write_keypair_file, Keypair, Signer},
        },
        solana_test_validator::{ProgramInfo, TestValidator, TestValidatorGenesis},
        spl_token_client::client::{
            ProgramClient, ProgramRpcClient, ProgramRpcClientSendTransaction,
        },
        std::path::PathBuf,
        tempfile::NamedTempFile,
    };

    fn clone_keypair(keypair: &Keypair) -> Keypair {
        Keypair::from_bytes(&keypair.to_bytes()).unwrap()
    }

    const TEST_DECIMALS: u8 = 0;

    async fn new_validator_for_test() -> (TestValidator, Keypair) {
        solana_logger::setup();
        let mut test_validator_genesis = TestValidatorGenesis::default();
        test_validator_genesis.add_programs_with_path(&[
            ProgramInfo {
                program_id: spl_token::id(),
                loader: bpf_loader::id(),
                program_path: PathBuf::from("../../target/deploy/spl_token.so"),
            },
            ProgramInfo {
                program_id: spl_associated_token_account::id(),
                loader: bpf_loader::id(),
                program_path: PathBuf::from("../../target/deploy/spl_associated_token_account.so"),
            },
            ProgramInfo {
                program_id: spl_token_2022::id(),
                loader: bpf_loader::id(),
                program_path: PathBuf::from("../../target/deploy/spl_token_2022.so"),
            },
        ]);
        test_validator_genesis.start_async().await
    }

    fn test_config<'a>(
        test_validator: &TestValidator,
        payer: &Keypair,
        program_id: &Pubkey,
    ) -> Config<'a> {
        let websocket_url = test_validator.rpc_pubsub_url();
        let rpc_client = Arc::new(test_validator.get_async_rpc_client());
        let program_client: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = Arc::new(
            ProgramRpcClient::new(rpc_client.clone(), ProgramRpcClientSendTransaction),
        );
        Config {
            rpc_client,
            program_client,
            websocket_url,
            output_format: OutputFormat::JsonCompact,
            fee_payer: Arc::new(clone_keypair(payer)),
            default_signer: Arc::new(clone_keypair(payer)),
            nonce_account: None,
            nonce_authority: None,
            sign_only: false,
            dump_transaction_message: false,
            multisigner_pubkeys: vec![],
            program_id: *program_id,
        }
    }

    async fn do_create_native_mint(config: &Config<'_>, program_id: &Pubkey, payer: &Keypair) {
        if program_id == &spl_token_2022::id() {
            let native_mint = spl_token_2022::native_mint::id();
            if config.rpc_client.get_account(&native_mint).await.is_err() {
                let transaction = Transaction::new_signed_with_payer(
                    &[create_native_mint(program_id, &payer.pubkey()).unwrap()],
                    Some(&payer.pubkey()),
                    &[payer],
                    config.rpc_client.get_latest_blockhash().await.unwrap(),
                );
                config
                    .rpc_client
                    .send_and_confirm_transaction(&transaction)
                    .await
                    .unwrap();
            }
        }
    }

    async fn create_token(config: &Config<'_>, payer: &Keypair) -> Pubkey {
        let token = Keypair::new();
        let token_pubkey = token.pubkey();
        let bulk_signers: Vec<Arc<dyn Signer>> =
            vec![Arc::new(clone_keypair(payer)), Arc::new(token)];

        command_create_token(
            config,
            TEST_DECIMALS,
            token_pubkey,
            payer.pubkey(),
            false,
            false,
            None,
            None,
            bulk_signers,
        )
        .await
        .unwrap();
        token_pubkey
    }

    async fn create_interest_bearing_token(
        config: &Config<'_>,
        payer: &Keypair,
        rate_bps: i16,
    ) -> Pubkey {
        let token = Keypair::new();
        let token_pubkey = token.pubkey();
        let bulk_signers: Vec<Arc<dyn Signer>> =
            vec![Arc::new(clone_keypair(payer)), Arc::new(token)];

        command_create_token(
            config,
            TEST_DECIMALS,
            token_pubkey,
            payer.pubkey(),
            false,
            false,
            None,
            Some(rate_bps),
            bulk_signers,
        )
        .await
        .unwrap();
        token_pubkey
    }

    async fn create_auxiliary_account(
        config: &Config<'_>,
        payer: &Keypair,
        mint: Pubkey,
    ) -> Pubkey {
        let auxiliary = Keypair::new();
        let address = auxiliary.pubkey();
        let bulk_signers: Vec<Arc<dyn Signer>> =
            vec![Arc::new(clone_keypair(payer)), Arc::new(auxiliary)];
        command_create_account(
            config,
            mint,
            payer.pubkey(),
            Some(address),
            false,
            bulk_signers,
        )
        .await
        .unwrap();
        address
    }

    async fn create_associated_account(
        config: &Config<'_>,
        payer: &Keypair,
        mint: Pubkey,
    ) -> Pubkey {
        let bulk_signers: Vec<Arc<dyn Signer>> = vec![Arc::new(clone_keypair(payer))];
        command_create_account(config, mint, payer.pubkey(), None, false, bulk_signers)
            .await
            .unwrap();
        get_associated_token_address_with_program_id(&payer.pubkey(), &mint, &config.program_id)
    }

    async fn mint_tokens(
        config: &Config<'_>,
        payer: &Keypair,
        mint: Pubkey,
        ui_amount: f64,
        recipient: Pubkey,
    ) {
        let bulk_signers: Vec<Arc<dyn Signer>> = vec![Arc::new(clone_keypair(payer))];
        command_mint(
            config,
            mint,
            ui_amount,
            recipient,
            MintInfo {
                program_id: config.program_id,
                address: mint,
                decimals: TEST_DECIMALS,
            },
            payer.pubkey(),
            false,
            None,
            bulk_signers,
        )
        .await
        .unwrap();
    }

    async fn process_test_command(
        config: &Config<'_>,
        payer: &Keypair,
        args: &[&str],
    ) -> CommandResult {
        let default_decimals = format!("{}", spl_token_2022::native_mint::DECIMALS);
        let minimum_signers_help = minimum_signers_help_string();
        let multisig_member_help = multisig_member_help_string();

        let app_matches = app(
            &default_decimals,
            &minimum_signers_help,
            &multisig_member_help,
        )
        .get_matches_from(args);
        let (sub_command, sub_matches) = app_matches.subcommand();
        let sub_command = CommandName::from_str(sub_command).unwrap();
        let matches = sub_matches.unwrap();

        let wallet_manager = None;
        let bulk_signers: Vec<Arc<dyn Signer>> = vec![Arc::new(clone_keypair(payer))];
        process_command(&sub_command, matches, config, wallet_manager, bulk_signers).await
    }

    async fn exec_test_cmd(config: &Config<'_>, args: &[&str]) -> CommandResult {
        let default_decimals = format!("{}", spl_token_2022::native_mint::DECIMALS);
        let minimum_signers_help = minimum_signers_help_string();
        let multisig_member_help = multisig_member_help_string();

        let app_matches = app(
            &default_decimals,
            &minimum_signers_help,
            &multisig_member_help,
        )
        .get_matches_from(args);
        let (sub_command, sub_matches) = app_matches.subcommand();
        let sub_command = CommandName::from_str(sub_command).unwrap();
        let matches = sub_matches.unwrap();

        let mut wallet_manager = None;
        let mut bulk_signers: Vec<Arc<dyn Signer>> = Vec::new();
        let mut multisigner_ids = Vec::new();

        let config = Config::new_with_clients_and_ws_url(
            matches,
            &mut wallet_manager,
            &mut bulk_signers,
            &mut multisigner_ids,
            config.rpc_client.clone(),
            config.program_client.clone(),
            config.websocket_url.clone(),
        )
        .await;

        process_command(&sub_command, matches, &config, wallet_manager, bulk_signers).await
    }

    #[tokio::test]
    #[serial]
    async fn create_token_default() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);
            let result = process_test_command(
                &config,
                &payer,
                &["spl-token", CommandName::CreateToken.into()],
            )
            .await;
            let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
            let mint =
                Pubkey::from_str(value["commandOutput"]["address"].as_str().unwrap()).unwrap();
            let account = config.rpc_client.get_account(&mint).await.unwrap();
            assert_eq!(account.owner, program_id);
        }
    }

    #[tokio::test]
    #[serial]
    async fn create_token_interest_bearing() {
        let (test_validator, payer) = new_validator_for_test().await;
        let config = test_config(&test_validator, &payer, &spl_token_2022::id());
        let rate_bps: i16 = 100;
        let result = process_test_command(
            &config,
            &payer,
            &[
                "spl-token",
                CommandName::CreateToken.into(),
                "--interest-rate",
                &rate_bps.to_string(),
            ],
        )
        .await;
        let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        let mint = Pubkey::from_str(value["commandOutput"]["address"].as_str().unwrap()).unwrap();
        let account = config.rpc_client.get_account(&mint).await.unwrap();
        let mint_account =
            StateWithExtensionsOwned::<spl_token_2022::state::Mint>::unpack(account.data).unwrap();
        let extension = mint_account
            .get_extension::<InterestBearingConfig>()
            .unwrap();
        assert_eq!(account.owner, spl_token_2022::id());
        assert_eq!(i16::from(extension.current_rate), rate_bps);
        assert_eq!(
            Option::<Pubkey>::from(extension.rate_authority),
            Some(payer.pubkey())
        );
    }

    #[tokio::test]
    #[serial]
    async fn set_interest_rate() {
        let (test_validator, payer) = new_validator_for_test().await;
        let config = test_config(&test_validator, &payer, &spl_token_2022::id());
        let initial_rate: i16 = 100;
        let new_rate: i16 = 300;
        let token = create_interest_bearing_token(&config, &payer, initial_rate).await;
        let account = config.rpc_client.get_account(&token).await.unwrap();
        let mint_account =
            StateWithExtensionsOwned::<spl_token_2022::state::Mint>::unpack(account.data).unwrap();
        let extension = mint_account
            .get_extension::<InterestBearingConfig>()
            .unwrap();
        assert_eq!(account.owner, spl_token_2022::id());
        assert_eq!(i16::from(extension.current_rate), initial_rate);

        let result = process_test_command(
            &config,
            &payer,
            &[
                "spl-token",
                CommandName::SetInterestRate.into(),
                &token.to_string(),
                &new_rate.to_string(),
            ],
        )
        .await;
        let _value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        let account = config.rpc_client.get_account(&token).await.unwrap();
        let mint_account =
            StateWithExtensionsOwned::<spl_token_2022::state::Mint>::unpack(account.data).unwrap();
        let extension = mint_account
            .get_extension::<InterestBearingConfig>()
            .unwrap();
        assert_eq!(i16::from(extension.current_rate), new_rate);
    }

    #[tokio::test]
    #[serial]
    async fn supply() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);
            let token = create_token(&config, &payer).await;
            let result = process_test_command(
                &config,
                &payer,
                &["spl-token", CommandName::Supply.into(), &token.to_string()],
            )
            .await;
            let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
            assert_eq!(value["amount"], "0");
            assert_eq!(value["uiAmountString"], "0");
        }
    }

    #[tokio::test]
    #[serial]
    async fn create_account_default() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);
            let token = create_token(&config, &payer).await;
            let result = process_test_command(
                &config,
                &payer,
                &[
                    "spl-token",
                    CommandName::CreateAccount.into(),
                    &token.to_string(),
                ],
            )
            .await;
            result.unwrap();
        }
    }

    #[tokio::test]
    #[serial]
    async fn account_info() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);
            let token = create_token(&config, &payer).await;
            let _account = create_associated_account(&config, &payer, token).await;
            let result = process_test_command(
                &config,
                &payer,
                &[
                    "spl-token",
                    CommandName::AccountInfo.into(),
                    &token.to_string(),
                ],
            )
            .await;
            let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
            let account = get_associated_token_address_with_program_id(
                &payer.pubkey(),
                &token,
                &config.program_id,
            );
            assert_eq!(value["address"], account.to_string());
            assert_eq!(value["mint"], token.to_string());
            assert_eq!(value["isAssociated"], true);
            assert_eq!(value["isNative"], false);
            assert_eq!(value["owner"], payer.pubkey().to_string());
            assert_eq!(value["state"], "initialized");
        }
    }

    #[tokio::test]
    #[serial]
    async fn balance() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);
            let token = create_token(&config, &payer).await;
            let _account = create_associated_account(&config, &payer, token).await;
            let result = process_test_command(
                &config,
                &payer,
                &["spl-token", CommandName::Balance.into(), &token.to_string()],
            )
            .await;
            let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
            assert_eq!(value["amount"], "0");
            assert_eq!(value["uiAmountString"], "0");
        }
    }

    #[tokio::test]
    #[serial]
    async fn mint() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);
            let token = create_token(&config, &payer).await;
            let account = create_associated_account(&config, &payer, token).await;
            let result = process_test_command(
                &config,
                &payer,
                &[
                    "spl-token",
                    CommandName::Mint.into(),
                    &token.to_string(),
                    "100",
                ],
            )
            .await;
            result.unwrap();
            let account = config.rpc_client.get_account(&account).await.unwrap();
            let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
            assert_eq!(token_account.base.amount, 100);
            assert_eq!(token_account.base.mint, token);
            assert_eq!(token_account.base.owner, payer.pubkey());
        }
    }

    #[tokio::test]
    #[serial]
    async fn balance_after_mint() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);
            let token = create_token(&config, &payer).await;
            let account = create_associated_account(&config, &payer, token).await;
            let ui_amount = 100.0;
            mint_tokens(&config, &payer, token, ui_amount, account).await;
            let result = process_test_command(
                &config,
                &payer,
                &["spl-token", CommandName::Balance.into(), &token.to_string()],
            )
            .await;
            let value: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
            assert_eq!(value["amount"], format!("{}", ui_amount));
            assert_eq!(value["uiAmountString"], format!("{}", ui_amount));
        }
    }

    #[tokio::test]
    #[serial]
    async fn accounts() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);
            let token1 = create_token(&config, &payer).await;
            let _account1 = create_associated_account(&config, &payer, token1).await;
            let token2 = create_token(&config, &payer).await;
            let _account2 = create_associated_account(&config, &payer, token2).await;
            let token3 = create_token(&config, &payer).await;
            let result = process_test_command(
                &config,
                &payer,
                &["spl-token", CommandName::Accounts.into()],
            )
            .await
            .unwrap();
            assert!(result.contains(&token1.to_string()));
            assert!(result.contains(&token2.to_string()));
            assert!(!result.contains(&token3.to_string()));
        }
    }

    #[tokio::test]
    #[serial]
    async fn wrap() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);
            let native_mint = *Token::new_native(
                config.program_client.clone(),
                &program_id,
                config.fee_payer.clone(),
            )
            .get_address();
            do_create_native_mint(&config, &program_id, &payer).await;
            let _result = process_test_command(
                &config,
                &payer,
                &["spl-token", CommandName::Wrap.into(), "0.5"],
            )
            .await
            .unwrap();
            let account = get_associated_token_address_with_program_id(
                &payer.pubkey(),
                &native_mint,
                &config.program_id,
            );
            let account = config.rpc_client.get_account(&account).await.unwrap();
            let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
            assert_eq!(token_account.base.mint, native_mint);
            assert_eq!(token_account.base.owner, payer.pubkey());
            assert!(token_account.base.is_native());
        }
    }

    #[tokio::test]
    #[serial]
    async fn unwrap() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);
            do_create_native_mint(&config, &program_id, &payer).await;
            let (signer, account) = new_throwaway_signer();
            let bulk_signers: Vec<Arc<dyn Signer>> = vec![Arc::new(clone_keypair(&payer)), signer];
            command_wrap(
                &config,
                0.5,
                payer.pubkey(),
                Some(account),
                false,
                bulk_signers,
            )
            .await
            .unwrap();
            let result = process_test_command(
                &config,
                &payer,
                &[
                    "spl-token",
                    CommandName::Unwrap.into(),
                    &account.to_string(),
                ],
            )
            .await;
            result.unwrap();
            config.rpc_client.get_account(&account).await.unwrap_err();
        }
    }

    #[tokio::test]
    #[serial]
    async fn transfer() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);
            let token = create_token(&config, &payer).await;
            let source = create_associated_account(&config, &payer, token).await;
            let destination = create_auxiliary_account(&config, &payer, token).await;
            let ui_amount = 100.0;
            mint_tokens(&config, &payer, token, ui_amount, source).await;
            let result = process_test_command(
                &config,
                &payer,
                &[
                    "spl-token",
                    CommandName::Transfer.into(),
                    &token.to_string(),
                    "10",
                    &destination.to_string(),
                ],
            )
            .await;
            result.unwrap();

            let account = config.rpc_client.get_account(&source).await.unwrap();
            let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
            assert_eq!(token_account.base.amount, 90);
            let account = config.rpc_client.get_account(&destination).await.unwrap();
            let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
            assert_eq!(token_account.base.amount, 10);
        }
    }

    #[tokio::test]
    #[serial]
    async fn transfer_fund_recipient() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);
            let token = create_token(&config, &payer).await;
            let source = create_associated_account(&config, &payer, token).await;
            let recipient = Keypair::new().pubkey().to_string();
            let ui_amount = 100.0;
            mint_tokens(&config, &payer, token, ui_amount, source).await;
            let result = process_test_command(
                &config,
                &payer,
                &[
                    "spl-token",
                    CommandName::Transfer.into(),
                    "--fund-recipient",
                    "--allow-unfunded-recipient",
                    &token.to_string(),
                    "10",
                    &recipient,
                ],
            )
            .await;
            result.unwrap();

            let account = config.rpc_client.get_account(&source).await.unwrap();
            let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
            assert_eq!(token_account.base.amount, 90);
        }
    }

    #[tokio::test]
    #[serial]
    async fn failing_to_allow_non_system_account_recipient() {
        let (test_validator, payer) = new_validator_for_test().await;
        let config = test_config(&test_validator, &payer, &spl_token::id());

        let token = create_token(&config, &payer).await;
        let source = create_associated_account(&config, &payer, token).await;
        let recipient = token.to_string();
        let ui_amount = 100.0;
        mint_tokens(&config, &payer, token, ui_amount, source).await;
        let result = process_test_command(
            &config,
            &payer,
            &[
                "spl-token",
                CommandName::Transfer.into(),
                "--fund-recipient",
                &token.to_string(),
                "10",
                &recipient,
            ],
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn allow_non_system_account_recipient() {
        let (test_validator, payer) = new_validator_for_test().await;
        let config = test_config(&test_validator, &payer, &spl_token::id());

        let token = create_token(&config, &payer).await;
        let source = create_associated_account(&config, &payer, token).await;
        let recipient = token.to_string();
        let ui_amount = 100.0;
        mint_tokens(&config, &payer, token, ui_amount, source).await;
        let result = process_test_command(
            &config,
            &payer,
            &[
                "spl-token",
                CommandName::Transfer.into(),
                "--fund-recipient",
                "--allow-non-system-account-recipient",
                "--allow-unfunded-recipient",
                &token.to_string(),
                "10",
                &recipient,
            ],
        )
        .await;
        result.unwrap();

        let ui_account = config
            .rpc_client
            .get_token_account(&source)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ui_account.token_amount.amount, "90");
    }

    #[tokio::test]
    #[serial]
    async fn close_wrapped_sol_account() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);
            let bulk_signers: Vec<Arc<dyn Signer>> = vec![Arc::new(clone_keypair(&payer))];

            let native_mint = *Token::new_native(
                config.program_client.clone(),
                &program_id,
                config.fee_payer.clone(),
            )
            .get_address();
            let token = create_token(&config, &payer).await;
            let source = create_associated_account(&config, &payer, token).await;
            do_create_native_mint(&config, &program_id, &payer).await;
            let ui_amount = 10.0;
            command_wrap(
                &config,
                ui_amount,
                payer.pubkey(),
                None,
                false,
                bulk_signers,
            )
            .await
            .unwrap();

            let recipient = get_associated_token_address_with_program_id(
                &payer.pubkey(),
                &native_mint,
                &program_id,
            );
            let result = process_test_command(
                &config,
                &payer,
                &[
                    "spl-token",
                    CommandName::Close.into(),
                    "--address",
                    &source.to_string(),
                    "--recipient",
                    &recipient.to_string(),
                ],
            )
            .await;
            result.unwrap();

            let ui_account = config
                .rpc_client
                .get_token_account(&recipient)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(ui_account.token_amount.amount, "10000000000");
        }
    }

    #[tokio::test]
    #[serial]
    async fn disable_mint_authority() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);
            let token = create_token(&config, &payer).await;
            let result = process_test_command(
                &config,
                &payer,
                &[
                    "spl-token",
                    CommandName::Authorize.into(),
                    &token.to_string(),
                    "mint",
                    "--disable",
                ],
            )
            .await;
            result.unwrap();

            let account = config.rpc_client.get_account(&token).await.unwrap();
            let mint = Mint::unpack(&account.data).unwrap();
            assert_eq!(mint.mint_authority, COption::None);
        }
    }

    #[tokio::test]
    #[serial]
    async fn gc() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let mut config = test_config(&test_validator, &payer, &program_id);
            let token = create_token(&config, &payer).await;
            let _account = create_associated_account(&config, &payer, token).await;
            let _aux1 = create_auxiliary_account(&config, &payer, token).await;
            let _aux2 = create_auxiliary_account(&config, &payer, token).await;
            let _aux3 = create_auxiliary_account(&config, &payer, token).await;
            let result = process_test_command(
                &config,
                &payer,
                &[
                    "spl-token",
                    CommandName::Accounts.into(),
                    &token.to_string(),
                ],
            )
            .await
            .unwrap();
            let value: serde_json::Value = serde_json::from_str(&result).unwrap();
            assert_eq!(value["accounts"].as_array().unwrap().len(), 4);
            config.output_format = OutputFormat::Display; // fixup eventually?
            let _result =
                process_test_command(&config, &payer, &["spl-token", CommandName::Gc.into()])
                    .await
                    .unwrap();
            config.output_format = OutputFormat::JsonCompact;
            let result = process_test_command(
                &config,
                &payer,
                &[
                    "spl-token",
                    CommandName::Accounts.into(),
                    &token.to_string(),
                ],
            )
            .await
            .unwrap();
            let value: serde_json::Value = serde_json::from_str(&result).unwrap();
            assert_eq!(value["accounts"].as_array().unwrap().len(), 1);
        }
    }

    #[tokio::test]
    #[serial]
    async fn set_owner() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);
            let token = create_token(&config, &payer).await;
            let aux = create_auxiliary_account(&config, &payer, token).await;
            let aux_string = aux.to_string();
            let _result = process_test_command(
                &config,
                &payer,
                &[
                    "spl-token",
                    CommandName::Authorize.into(),
                    &aux_string,
                    "owner",
                    &aux_string,
                ],
            )
            .await
            .unwrap();
            let account = config.rpc_client.get_account(&aux).await.unwrap();
            let token_account = StateWithExtensionsOwned::<Account>::unpack(account.data).unwrap();
            assert_eq!(token_account.base.mint, token);
            assert_eq!(token_account.base.owner, aux);
        }
    }

    #[tokio::test]
    #[serial]
    async fn transfer_with_account_delegate() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);

            let token = create_token(&config, &payer).await;
            let source = create_associated_account(&config, &payer, token).await;
            let destination = create_auxiliary_account(&config, &payer, token).await;
            let delegate = Keypair::new();

            let delegate_keypair_file = NamedTempFile::new().unwrap();
            write_keypair_file(&delegate, &delegate_keypair_file).unwrap();
            let fee_payer_keypair_file = NamedTempFile::new().unwrap();
            write_keypair_file(&payer, &fee_payer_keypair_file).unwrap();

            let ui_amount = 100.0;
            mint_tokens(&config, &payer, token, ui_amount, source).await;

            let ui_account = config
                .rpc_client
                .get_token_account(&source)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(ui_account.token_amount.amount, "100");
            assert_eq!(ui_account.delegate, None);
            assert_eq!(ui_account.delegated_amount, None);
            let ui_account = config
                .rpc_client
                .get_token_account(&destination)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(ui_account.token_amount.amount, "0");

            exec_test_cmd(
                &config,
                &[
                    "spl-token",
                    CommandName::Approve.into(),
                    &source.to_string(),
                    "10",
                    &delegate.pubkey().to_string(),
                    "--owner",
                    fee_payer_keypair_file.path().to_str().unwrap(),
                    "--fee-payer",
                    fee_payer_keypair_file.path().to_str().unwrap(),
                    "--program-id",
                    &program_id.to_string(),
                ],
            )
            .await
            .unwrap();

            let ui_account = config
                .rpc_client
                .get_token_account(&source)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(ui_account.delegate.unwrap(), delegate.pubkey().to_string());
            assert_eq!(ui_account.delegated_amount.unwrap().amount, "10");

            let result = exec_test_cmd(
                &config,
                &[
                    "spl-token",
                    CommandName::Transfer.into(),
                    &token.to_string(),
                    "10",
                    &destination.to_string(),
                    "--from",
                    &source.to_string(),
                    "--owner",
                    delegate_keypair_file.path().to_str().unwrap(),
                    "--fee-payer",
                    fee_payer_keypair_file.path().to_str().unwrap(),
                    "--program-id",
                    &program_id.to_string(),
                ],
            )
            .await;
            result.unwrap();

            let ui_account = config
                .rpc_client
                .get_token_account(&source)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(ui_account.token_amount.amount, "90");
            assert_eq!(ui_account.delegate, None);
            assert_eq!(ui_account.delegated_amount, None);
            let ui_account = config
                .rpc_client
                .get_token_account(&destination)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(ui_account.token_amount.amount, "10");
        }
    }

    #[tokio::test]
    #[serial]
    async fn burn_with_account_delegate() {
        let (test_validator, payer) = new_validator_for_test().await;
        for program_id in [spl_token::id(), spl_token_2022::id()] {
            let config = test_config(&test_validator, &payer, &program_id);

            let token = create_token(&config, &payer).await;
            let source = create_associated_account(&config, &payer, token).await;
            let delegate = Keypair::new();

            let delegate_keypair_file = NamedTempFile::new().unwrap();
            write_keypair_file(&delegate, &delegate_keypair_file).unwrap();
            let fee_payer_keypair_file = NamedTempFile::new().unwrap();
            write_keypair_file(&payer, &fee_payer_keypair_file).unwrap();

            let ui_amount = 100.0;
            mint_tokens(&config, &payer, token, ui_amount, source).await;

            let ui_account = config
                .rpc_client
                .get_token_account(&source)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(ui_account.token_amount.amount, "100");
            assert_eq!(ui_account.delegate, None);
            assert_eq!(ui_account.delegated_amount, None);

            exec_test_cmd(
                &config,
                &[
                    "spl-token",
                    CommandName::Approve.into(),
                    &source.to_string(),
                    "10",
                    &delegate.pubkey().to_string(),
                    "--owner",
                    fee_payer_keypair_file.path().to_str().unwrap(),
                    "--fee-payer",
                    fee_payer_keypair_file.path().to_str().unwrap(),
                    "--program-id",
                    &program_id.to_string(),
                ],
            )
            .await
            .unwrap();

            let ui_account = config
                .rpc_client
                .get_token_account(&source)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(ui_account.delegate.unwrap(), delegate.pubkey().to_string());
            assert_eq!(ui_account.delegated_amount.unwrap().amount, "10");

            let result = exec_test_cmd(
                &config,
                &[
                    "spl-token",
                    CommandName::Burn.into(),
                    &source.to_string(),
                    "10",
                    "--owner",
                    delegate_keypair_file.path().to_str().unwrap(),
                    "--fee-payer",
                    fee_payer_keypair_file.path().to_str().unwrap(),
                    "--program-id",
                    &program_id.to_string(),
                ],
            )
            .await;
            result.unwrap();

            let ui_account = config
                .rpc_client
                .get_token_account(&source)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(ui_account.token_amount.amount, "90");
            assert_eq!(ui_account.delegate, None);
            assert_eq!(ui_account.delegated_amount, None);
        }
    }

    #[tokio::test]
    #[serial]
    async fn close_mint() {
        let (test_validator, payer) = new_validator_for_test().await;
        let config = test_config(&test_validator, &payer, &spl_token_2022::id());

        let token_keypair = Keypair::new();
        let token_pubkey = token_keypair.pubkey();
        let bulk_signers: Vec<Arc<dyn Signer>> =
            vec![Arc::new(clone_keypair(&payer)), Arc::new(token_keypair)];

        command_create_token(
            &config,
            TEST_DECIMALS,
            token_pubkey,
            payer.pubkey(),
            false,
            true,
            None,
            None,
            bulk_signers,
        )
        .await
        .unwrap();

        let account = config.rpc_client.get_account(&token_pubkey).await.unwrap();
        let test_mint = StateWithExtensionsOwned::<Mint>::unpack(account.data);
        assert!(test_mint.is_ok());

        process_test_command(
            &config,
            &payer,
            &[
                "spl-token",
                CommandName::CloseMint.into(),
                &token_pubkey.to_string(),
            ],
        )
        .await
        .unwrap();

        let account = config.rpc_client.get_account(&token_pubkey).await;
        assert!(account.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn required_transfer_memos() {
        let (test_validator, payer) = new_validator_for_test().await;
        let program_id = spl_token_2022::id();
        let config = test_config(&test_validator, &payer, &program_id);
        let token = create_token(&config, &payer).await;
        let token_account = create_associated_account(&config, &payer, token).await;
        let result = process_test_command(
            &config,
            &payer,
            &[
                "spl-token",
                CommandName::EnableRequiredTransferMemos.into(),
                &token_account.to_string(),
            ],
        )
        .await;
        result.unwrap();
        let extensions = StateWithExtensionsOwned::<Account>::unpack(
            config
                .rpc_client
                .get_account(&token_account)
                .await
                .unwrap()
                .data,
        )
        .unwrap();
        let memo_transfer = extensions.get_extension::<MemoTransfer>().unwrap();
        let enabled: bool = memo_transfer.require_incoming_transfer_memos.into();
        assert!(enabled);
        let result = process_test_command(
            &config,
            &payer,
            &[
                "spl-token",
                CommandName::DisableRequiredTransferMemos.into(),
                &token_account.to_string(),
            ],
        )
        .await;
        result.unwrap();
        let extensions = StateWithExtensionsOwned::<Account>::unpack(
            config
                .rpc_client
                .get_account(&token_account)
                .await
                .unwrap()
                .data,
        )
        .unwrap();
        let memo_transfer = extensions.get_extension::<MemoTransfer>().unwrap();
        let enabled: bool = memo_transfer.require_incoming_transfer_memos.into();
        assert!(!enabled);
    }

    #[tokio::test]
    #[serial]
    async fn immutable_accounts() {
        let (test_validator, payer) = new_validator_for_test().await;
        let program_id = spl_token_2022::id();
        let config = test_config(&test_validator, &payer, &program_id);
        let token = create_token(&config, &payer).await;
        let new_owner = Keypair::new().pubkey();
        let bulk_signers: Vec<Arc<dyn Signer>> = vec![Arc::new(clone_keypair(&payer))];
        let native_mint = *Token::new_native(
            config.program_client.clone(),
            &program_id,
            config.fee_payer.clone(),
        )
        .get_address();
        do_create_native_mint(&config, &program_id, &payer).await;

        // cannot reassign an ata
        let account = create_associated_account(&config, &payer, token).await;
        let result = command_authorize(
            &config,
            account,
            AuthorityType::AccountOwner,
            payer.pubkey(),
            Some(new_owner),
            true,
            bulk_signers.clone(),
        )
        .await;
        result.unwrap_err();

        // immutable works for create-account
        let aux_account = Keypair::new();
        let aux_pubkey = aux_account.pubkey();
        let aux_keypair_file = NamedTempFile::new().unwrap();
        write_keypair_file(&aux_account, &aux_keypair_file).unwrap();

        process_test_command(
            &config,
            &payer,
            &[
                "spl-token",
                CommandName::CreateAccount.into(),
                &token.to_string(),
                aux_keypair_file.path().to_str().unwrap(),
                "--immutable",
            ],
        )
        .await
        .unwrap();

        let result = command_authorize(
            &config,
            aux_pubkey,
            AuthorityType::AccountOwner,
            payer.pubkey(),
            Some(new_owner),
            true,
            bulk_signers.clone(),
        )
        .await;
        result.unwrap_err();

        // immutable works for wrap
        process_test_command(
            &config,
            &payer,
            &[
                "spl-token",
                CommandName::Wrap.into(),
                "--create-aux-account",
                "--immutable",
                "0.5",
            ],
        )
        .await
        .unwrap();

        let accounts = config
            .rpc_client
            .get_token_accounts_by_owner(&payer.pubkey(), TokenAccountsFilter::Mint(native_mint))
            .await
            .unwrap();

        let result = command_authorize(
            &config,
            Pubkey::from_str(&accounts[0].pubkey).unwrap(),
            AuthorityType::AccountOwner,
            payer.pubkey(),
            Some(new_owner),
            true,
            bulk_signers.clone(),
        )
        .await;
        result.unwrap_err();
    }
}
