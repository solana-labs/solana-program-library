#![allow(deprecated)]

use {
    clap::{
        crate_description, crate_name, crate_version, App, AppSettings, Arg, ArgGroup, SubCommand,
    },
    solana_clap_v3_utils::{
        fee_payer::fee_payer_arg,
        input_parsers::Amount,
        input_validators::{is_pubkey, is_url_or_moniker, is_valid_pubkey, is_valid_signer},
        memo::memo_arg,
        nonce::*,
        offline::{self, *},
        ArgConstant,
    },
    solana_sdk::{instruction::AccountMeta, pubkey::Pubkey},
    spl_token_2022::instruction::{AuthorityType, MAX_SIGNERS, MIN_SIGNERS},
    std::{fmt, str::FromStr},
    strum::IntoEnumIterator,
    strum_macros::{AsRefStr, EnumIter, EnumString, IntoStaticStr},
};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub const OWNER_ADDRESS_ARG: ArgConstant<'static> = ArgConstant {
    name: "owner",
    long: "owner",
    help: "Address of the primary authority controlling a mint or account. Defaults to the client keypair address.",
};

pub const OWNER_KEYPAIR_ARG: ArgConstant<'static> = ArgConstant {
    name: "owner",
    long: "owner",
    help: "Keypair of the primary authority controlling a mint or account. Defaults to the client keypair.",
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

pub const TRANSFER_LAMPORTS_ARG: ArgConstant<'static> = ArgConstant {
    name: "transfer_lamports",
    long: "transfer-lamports",
    help: "Additional lamports to transfer to make account rent-exempt after reallocation. Required by --sign-only",
};

pub const MULTISIG_SIGNER_ARG: ArgConstant<'static> = ArgConstant {
    name: "multisig_signer",
    long: "multisig-signer",
    help: "Member signer of a multisig account",
};

pub const COMPUTE_UNIT_PRICE_ARG: ArgConstant<'static> = ArgConstant {
    name: "compute_unit_price",
    long: "--with-compute-unit-price",
    help: "Set compute unit price for transaction, in increments of 0.000001 lamports per compute unit.",
};

pub const COMPUTE_UNIT_LIMIT_ARG: ArgConstant<'static> = ArgConstant {
    name: "compute_unit_limit",
    long: "--with-compute-unit-limit",
    help: "Set compute unit limit for transaction, in compute units.",
};

// The `signer_arg` in clap-v3-utils` specifies the argument as a
// `PubkeySignature` type, but supporting `PubkeySignature` in the token-cli
// requires a significant re-structuring of the code. Therefore, hard-code the
// `signer_arg` and `OfflineArgs` from clap-utils` here and remove
// it in a subsequent PR.
fn signer_arg<'a>() -> Arg<'a> {
    Arg::new(SIGNER_ARG.name)
        .long(SIGNER_ARG.long)
        .takes_value(true)
        .value_name("PUBKEY=SIGNATURE")
        .requires(BLOCKHASH_ARG.name)
        .action(clap::ArgAction::Append)
        .multiple_values(false)
        .help(SIGNER_ARG.help)
}

pub trait OfflineArgs {
    fn offline_args(self) -> Self;
    fn offline_args_config(self, config: &dyn ArgsConfig) -> Self;
}

impl OfflineArgs for clap::Command<'_> {
    fn offline_args_config(self, config: &dyn ArgsConfig) -> Self {
        self.arg(config.blockhash_arg(blockhash_arg()))
            .arg(config.sign_only_arg(sign_only_arg()))
            .arg(config.signer_arg(signer_arg()))
            .arg(config.dump_transaction_message_arg(dump_transaction_message()))
    }
    fn offline_args(self) -> Self {
        struct NullArgsConfig {}
        impl ArgsConfig for NullArgsConfig {}
        self.offline_args_config(&NullArgsConfig {})
    }
}

pub static VALID_TOKEN_PROGRAM_IDS: [Pubkey; 2] = [spl_token_2022::ID, spl_token::ID];

#[derive(AsRefStr, Debug, Clone, Copy, PartialEq, EnumString, IntoStaticStr)]
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
    EnableCpiGuard,
    DisableCpiGuard,
    UpdateDefaultAccountState,
    UpdateMetadataAddress,
    WithdrawWithheldTokens,
    SetTransferFee,
    WithdrawExcessLamports,
    SetTransferHook,
    InitializeMetadata,
    UpdateMetadata,
    InitializeGroup,
    UpdateGroupMaxSize,
    InitializeMember,
    UpdateConfidentialTransferSettings,
    ConfigureConfidentialTransferAccount,
    EnableConfidentialCredits,
    DisableConfidentialCredits,
    EnableNonConfidentialCredits,
    DisableNonConfidentialCredits,
    DepositConfidentialTokens,
    WithdrawConfidentialTokens,
    ApplyPendingBalance,
    UpdateGroupAddress,
    UpdateMemberAddress,
    MintConfidentialTokens,
    BurnConfidentialTokens,
    ConfidentialBalance,
    ConfidentialSupply,
    RotateSupplyElgamal,
}
impl fmt::Display for CommandName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum AccountMetaRole {
    Readonly,
    Writable,
    ReadonlySigner,
    WritableSigner,
}
impl fmt::Display for AccountMetaRole {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
pub fn parse_transfer_hook_account<T>(string: T) -> Result<AccountMeta, String>
where
    T: AsRef<str> + fmt::Display,
{
    match string.as_ref().split(':').collect::<Vec<_>>().as_slice() {
        [address, role] => {
            let address = Pubkey::from_str(address).map_err(|e| format!("{e}"))?;
            let meta = match AccountMetaRole::from_str(role).map_err(|e| format!("{e}"))? {
                AccountMetaRole::Readonly => AccountMeta::new_readonly(address, false),
                AccountMetaRole::Writable => AccountMeta::new(address, false),
                AccountMetaRole::ReadonlySigner => AccountMeta::new_readonly(address, true),
                AccountMetaRole::WritableSigner => AccountMeta::new(address, true),
            };
            Ok(meta)
        }
        _ => Err("Transfer hook account must be present as <ADDRESS>:<ROLE>".to_string()),
    }
}
fn validate_transfer_hook_account<T>(string: T) -> Result<(), String>
where
    T: AsRef<str> + fmt::Display,
{
    match string.as_ref().split(':').collect::<Vec<_>>().as_slice() {
        [address, role] => {
            is_valid_pubkey(address)?;
            AccountMetaRole::from_str(role)
                .map(|_| ())
                .map_err(|e| format!("{e}"))
        }
        _ => Err("Transfer hook account must be present as <ADDRESS>:<ROLE>".to_string()),
    }
}
#[derive(Debug, Clone, PartialEq, EnumIter, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
pub enum CliAuthorityType {
    Mint,
    Freeze,
    Owner,
    Close,
    CloseMint,
    TransferFeeConfig,
    WithheldWithdraw,
    InterestRate,
    PermanentDelegate,
    ConfidentialTransferMint,
    TransferHookProgramId,
    ConfidentialTransferFee,
    MetadataPointer,
    Metadata,
    GroupPointer,
    GroupMemberPointer,
    Group,
}
impl TryFrom<CliAuthorityType> for AuthorityType {
    type Error = Error;
    fn try_from(authority_type: CliAuthorityType) -> Result<Self, Error> {
        match authority_type {
            CliAuthorityType::Mint => Ok(AuthorityType::MintTokens),
            CliAuthorityType::Freeze => Ok(AuthorityType::FreezeAccount),
            CliAuthorityType::Owner => Ok(AuthorityType::AccountOwner),
            CliAuthorityType::Close => Ok(AuthorityType::CloseAccount),
            CliAuthorityType::CloseMint => Ok(AuthorityType::CloseMint),
            CliAuthorityType::TransferFeeConfig => Ok(AuthorityType::TransferFeeConfig),
            CliAuthorityType::WithheldWithdraw => Ok(AuthorityType::WithheldWithdraw),
            CliAuthorityType::InterestRate => Ok(AuthorityType::InterestRate),
            CliAuthorityType::PermanentDelegate => Ok(AuthorityType::PermanentDelegate),
            CliAuthorityType::ConfidentialTransferMint => {
                Ok(AuthorityType::ConfidentialTransferMint)
            }
            CliAuthorityType::TransferHookProgramId => Ok(AuthorityType::TransferHookProgramId),
            CliAuthorityType::ConfidentialTransferFee => {
                Ok(AuthorityType::ConfidentialTransferFeeConfig)
            }
            CliAuthorityType::MetadataPointer => Ok(AuthorityType::MetadataPointer),
            CliAuthorityType::Metadata => {
                Err("Metadata authority does not map to a token authority type".into())
            }
            CliAuthorityType::GroupPointer => Ok(AuthorityType::GroupPointer),
            CliAuthorityType::GroupMemberPointer => Ok(AuthorityType::GroupMemberPointer),
            CliAuthorityType::Group => {
                Err("Group update authority does not map to a token authority type".into())
            }
        }
    }
}

pub fn owner_address_arg<'a>() -> Arg<'a> {
    Arg::with_name(OWNER_ADDRESS_ARG.name)
        .long(OWNER_ADDRESS_ARG.long)
        .takes_value(true)
        .value_name("OWNER_ADDRESS")
        .validator(|s| is_valid_pubkey(s))
        .help(OWNER_ADDRESS_ARG.help)
}

pub fn owner_keypair_arg_with_value_name<'a>(value_name: &'static str) -> Arg<'a> {
    Arg::with_name(OWNER_KEYPAIR_ARG.name)
        .long(OWNER_KEYPAIR_ARG.long)
        .takes_value(true)
        .value_name(value_name)
        .validator(|s| is_valid_signer(s))
        .help(OWNER_KEYPAIR_ARG.help)
}

pub fn owner_keypair_arg<'a>() -> Arg<'a> {
    owner_keypair_arg_with_value_name("OWNER_KEYPAIR")
}

pub fn mint_address_arg<'a>() -> Arg<'a> {
    Arg::with_name(MINT_ADDRESS_ARG.name)
        .long(MINT_ADDRESS_ARG.long)
        .takes_value(true)
        .value_name("MINT_ADDRESS")
        .validator(|s| is_valid_pubkey(s))
        .help(MINT_ADDRESS_ARG.help)
}

pub fn mint_decimals_arg<'a>() -> Arg<'a> {
    Arg::with_name(MINT_DECIMALS_ARG.name)
        .long(MINT_DECIMALS_ARG.long)
        .takes_value(true)
        .value_name("MINT_DECIMALS")
        .value_parser(clap::value_parser!(u8))
        .help(MINT_DECIMALS_ARG.help)
}

pub trait MintArgs {
    fn mint_args(self) -> Self;
}

impl MintArgs for App<'_> {
    fn mint_args(self) -> Self {
        self.arg(mint_address_arg().requires(MINT_DECIMALS_ARG.name))
            .arg(mint_decimals_arg().requires(MINT_ADDRESS_ARG.name))
    }
}

pub fn delegate_address_arg<'a>() -> Arg<'a> {
    Arg::with_name(DELEGATE_ADDRESS_ARG.name)
        .long(DELEGATE_ADDRESS_ARG.long)
        .takes_value(true)
        .value_name("DELEGATE_ADDRESS")
        .validator(|s| is_valid_pubkey(s))
        .help(DELEGATE_ADDRESS_ARG.help)
}

pub fn transfer_lamports_arg<'a>() -> Arg<'a> {
    Arg::with_name(TRANSFER_LAMPORTS_ARG.name)
        .long(TRANSFER_LAMPORTS_ARG.long)
        .takes_value(true)
        .value_name("LAMPORTS")
        .value_parser(clap::value_parser!(u64))
        .help(TRANSFER_LAMPORTS_ARG.help)
}

pub fn multisig_signer_arg<'a>() -> Arg<'a> {
    Arg::with_name(MULTISIG_SIGNER_ARG.name)
        .long(MULTISIG_SIGNER_ARG.long)
        .validator(|s| is_valid_signer(s))
        .value_name("MULTISIG_SIGNER")
        .takes_value(true)
        .multiple(true)
        .min_values(0_usize)
        .max_values(MAX_SIGNERS)
        .help(MULTISIG_SIGNER_ARG.help)
}

fn is_multisig_minimum_signers(string: &str) -> Result<(), String> {
    let v = u8::from_str(string).map_err(|e| e.to_string())? as usize;
    if v < MIN_SIGNERS {
        Err(format!("must be at least {}", MIN_SIGNERS))
    } else if v > MAX_SIGNERS {
        Err(format!("must be at most {}", MAX_SIGNERS))
    } else {
        Ok(())
    }
}

fn is_valid_token_program_id<T>(string: T) -> Result<(), String>
where
    T: AsRef<str> + fmt::Display,
{
    match is_pubkey(string.as_ref()) {
        Ok(()) => {
            let program_id = string.as_ref().parse::<Pubkey>().unwrap();
            if VALID_TOKEN_PROGRAM_IDS.contains(&program_id) {
                Ok(())
            } else {
                Err(format!("Unrecognized token program id: {}", program_id))
            }
        }
        Err(e) => Err(e),
    }
}

struct SignOnlyNeedsFullMintSpec {}
impl offline::ArgsConfig for SignOnlyNeedsFullMintSpec {
    fn sign_only_arg<'a, 'b>(&self, arg: Arg<'a>) -> Arg<'a> {
        arg.requires_all(&[MINT_ADDRESS_ARG.name, MINT_DECIMALS_ARG.name])
    }
    fn signer_arg<'a, 'b>(&self, arg: Arg<'a>) -> Arg<'a> {
        arg.requires_all(&[MINT_ADDRESS_ARG.name, MINT_DECIMALS_ARG.name])
    }
}

struct SignOnlyNeedsMintDecimals {}
impl offline::ArgsConfig for SignOnlyNeedsMintDecimals {
    fn sign_only_arg<'a, 'b>(&self, arg: Arg<'a>) -> Arg<'a> {
        arg.requires_all(&[MINT_DECIMALS_ARG.name])
    }
    fn signer_arg<'a, 'b>(&self, arg: Arg<'a>) -> Arg<'a> {
        arg.requires_all(&[MINT_DECIMALS_ARG.name])
    }
}

struct SignOnlyNeedsMintAddress {}
impl offline::ArgsConfig for SignOnlyNeedsMintAddress {
    fn sign_only_arg<'a, 'b>(&self, arg: Arg<'a>) -> Arg<'a> {
        arg.requires_all(&[MINT_ADDRESS_ARG.name])
    }
    fn signer_arg<'a, 'b>(&self, arg: Arg<'a>) -> Arg<'a> {
        arg.requires_all(&[MINT_ADDRESS_ARG.name])
    }
}

struct SignOnlyNeedsDelegateAddress {}
impl offline::ArgsConfig for SignOnlyNeedsDelegateAddress {
    fn sign_only_arg<'a, 'b>(&self, arg: Arg<'a>) -> Arg<'a> {
        arg.requires_all(&[DELEGATE_ADDRESS_ARG.name])
    }
    fn signer_arg<'a, 'b>(&self, arg: Arg<'a>) -> Arg<'a> {
        arg.requires_all(&[DELEGATE_ADDRESS_ARG.name])
    }
}

struct SignOnlyNeedsTransferLamports {}
impl offline::ArgsConfig for SignOnlyNeedsTransferLamports {
    fn sign_only_arg<'a, 'b>(&self, arg: Arg<'a>) -> Arg<'a> {
        arg.requires_all(&[TRANSFER_LAMPORTS_ARG.name])
    }
    fn signer_arg<'a, 'b>(&self, arg: Arg<'a>) -> Arg<'a> {
        arg.requires_all(&[TRANSFER_LAMPORTS_ARG.name])
    }
}

pub fn minimum_signers_help_string() -> String {
    format!(
        "The minimum number of signers required to allow the operation. [{} <= M <= N]",
        MIN_SIGNERS
    )
}

pub fn multisig_member_help_string() -> String {
    format!(
        "The public keys for each of the N signing members of this account. [{} <= N <= {}]",
        MIN_SIGNERS, MAX_SIGNERS
    )
}

pub(crate) trait BenchSubCommand {
    fn bench_subcommand(self) -> Self;
}

impl BenchSubCommand for App<'_> {
    fn bench_subcommand(self) -> Self {
        self.subcommand(
            SubCommand::with_name("bench")
                .about("Token benchmarking facilities")
                .setting(AppSettings::InferSubcommands)
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("create-accounts")
                        .about("Create multiple token accounts for benchmarking")
                        .arg(
                            Arg::with_name("token")
                                .validator(|s| is_valid_pubkey(s))
                                .value_name("TOKEN_ADDRESS")
                                .takes_value(true)
                                .index(1)
                                .required(true)
                                .help("The token that the accounts will hold"),
                        )
                        .arg(
                            Arg::with_name("n")
                                .value_parser(clap::value_parser!(usize))
                                .value_name("N")
                                .takes_value(true)
                                .index(2)
                                .required(true)
                                .help("The number of accounts to create"),
                        )
                        .arg(owner_address_arg()),
                )
                .subcommand(
                    SubCommand::with_name("close-accounts")
                        .about("Close multiple token accounts used for benchmarking")
                        .arg(
                            Arg::with_name("token")
                                .validator(|s| is_valid_pubkey(s))
                                .value_name("TOKEN_ADDRESS")
                                .takes_value(true)
                                .index(1)
                                .required(true)
                                .help("The token that the accounts held"),
                        )
                        .arg(
                            Arg::with_name("n")
                                .value_parser(clap::value_parser!(usize))
                                .value_name("N")
                                .takes_value(true)
                                .index(2)
                                .required(true)
                                .help("The number of accounts to close"),
                        )
                        .arg(owner_address_arg()),
                )
                .subcommand(
                    SubCommand::with_name("deposit-into")
                        .about("Deposit tokens into multiple accounts")
                        .arg(
                            Arg::with_name("token")
                                .validator(|s| is_valid_pubkey(s))
                                .value_name("TOKEN_ADDRESS")
                                .takes_value(true)
                                .index(1)
                                .required(true)
                                .help("The token that the accounts will hold"),
                        )
                        .arg(
                            Arg::with_name("n")
                                .value_parser(clap::value_parser!(usize))
                                .value_name("N")
                                .takes_value(true)
                                .index(2)
                                .required(true)
                                .help("The number of accounts to deposit into"),
                        )
                        .arg(
                            Arg::with_name("amount")
                                .value_parser(Amount::parse)
                                .value_name("TOKEN_AMOUNT")
                                .takes_value(true)
                                .index(3)
                                .required(true)
                                .help("Amount to deposit into each account, in tokens"),
                        )
                        .arg(
                            Arg::with_name("from")
                                .long("from")
                                .validator(|s| is_valid_pubkey(s))
                                .value_name("SOURCE_TOKEN_ACCOUNT_ADDRESS")
                                .takes_value(true)
                                .help("The source token account address [default: associated token account for --owner]")
                        )
                        .arg(owner_address_arg()),
                )
                .subcommand(
                    SubCommand::with_name("withdraw-from")
                        .about("Withdraw tokens from multiple accounts")
                        .arg(
                            Arg::with_name("token")
                                .validator(|s| is_valid_pubkey(s))
                                .value_name("TOKEN_ADDRESS")
                                .takes_value(true)
                                .index(1)
                                .required(true)
                                .help("The token that the accounts hold"),
                        )
                        .arg(
                            Arg::with_name("n")
                                .value_parser(clap::value_parser!(usize))
                                .value_name("N")
                                .takes_value(true)
                                .index(2)
                                .required(true)
                                .help("The number of accounts to withdraw from"),
                        )
                        .arg(
                            Arg::with_name("amount")
                                .value_parser(Amount::parse)
                                .value_name("TOKEN_AMOUNT")
                                .takes_value(true)
                                .index(3)
                                .required(true)
                                .help("Amount to withdraw from each account, in tokens"),
                        )
                        .arg(
                            Arg::with_name("to")
                                .long("to")
                                .validator(|s| is_valid_pubkey(s))
                                .value_name("RECIPIENT_TOKEN_ACCOUNT_ADDRESS")
                                .takes_value(true)
                                .help("The recipient token account address [default: associated token account for --owner]")
                        )
                        .arg(owner_address_arg()),
                ),
        )
    }
}

pub fn app<'a>(
    default_decimals: &'a str,
    minimum_signers_help: &'a str,
    multisig_member_help: &'a str,
) -> App<'a> {
    App::new(crate_name!())
        .about(crate_description!())
        .version(crate_version!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .arg(
            Arg::with_name("config_file")
                .short('C')
                .long("config")
                .value_name("PATH")
                .takes_value(true)
                .global(true)
                .help("Configuration file to use"),
        )
        .arg(
            Arg::with_name("verbose")
                .short('v')
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
                .possible_values(["json", "json-compact"])
                .help("Return information in specified output format"),
        )
        .arg(
            Arg::with_name("program_2022")
                .long("program-2022")
                .takes_value(false)
                .global(true)
                .conflicts_with("program_id")
                .help("Use token extension program token 2022 with program id: TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb"),
        )
        .arg(
            Arg::with_name("program_id")
                .short('p')
                .long("program-id")
                .value_name("ADDRESS")
                .takes_value(true)
                .global(true)
                .conflicts_with("program_2022")
                .validator(|s| is_valid_token_program_id(s))
                .help("SPL Token program id"),
        )
        .arg(
            Arg::with_name("json_rpc_url")
                .short('u')
                .long("url")
                .value_name("URL_OR_MONIKER")
                .takes_value(true)
                .global(true)
                .validator(|s| is_url_or_moniker(s))
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
        .arg(
            Arg::with_name(COMPUTE_UNIT_LIMIT_ARG.name)
                .long(COMPUTE_UNIT_LIMIT_ARG.long)
                .takes_value(true)
                .global(true)
                .value_name("COMPUTE-UNIT-LIMIT")
                .value_parser(clap::value_parser!(u32))
                .help(COMPUTE_UNIT_LIMIT_ARG.help)
        )
        .arg(
            Arg::with_name(COMPUTE_UNIT_PRICE_ARG.name)
                .long(COMPUTE_UNIT_PRICE_ARG.long)
                .takes_value(true)
                .global(true)
                .value_name("COMPUTE-UNIT-PRICE")
                .value_parser(clap::value_parser!(u64))
                .help(COMPUTE_UNIT_PRICE_ARG.help)
        )
        .bench_subcommand()
        .subcommand(SubCommand::with_name(CommandName::CreateToken.into()).about("Create a new token")
                .arg(
                    Arg::with_name("token_keypair")
                        .value_name("TOKEN_KEYPAIR")
                        .validator(|s| is_valid_signer(s))
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
                        .validator(|s| is_valid_pubkey(s))
                        .takes_value(true)
                        .help(
                            "Specify the mint authority address. \
                             Defaults to the client keypair address."
                        ),
                )
                .arg(
                    Arg::with_name("decimals")
                        .long("decimals")
                        .value_parser(clap::value_parser!(u8))
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
                .arg(
                    Arg::with_name("metadata_address")
                        .long("metadata-address")
                        .value_name("ADDRESS")
                        .validator(|s| is_valid_pubkey(s))
                        .takes_value(true)
                        .conflicts_with("enable_metadata")
                        .help(
                            "Specify address that stores token metadata."
                        ),
                )
                .arg(
                    Arg::with_name("group_address")
                        .long("group-address")
                        .value_name("ADDRESS")
                        .validator(|s| is_valid_pubkey(s))
                        .takes_value(true)
                        .conflicts_with("enable_group")
                        .help(
                            "Specify address that stores token group configurations."
                        ),
                )
                .arg(
                    Arg::with_name("member_address")
                        .long("member-address")
                        .value_name("ADDRESS")
                        .validator(|s| is_valid_pubkey(s))
                        .takes_value(true)
                        .conflicts_with("enable_member")
                        .help(
                            "Specify address that stores token member configurations."
                        ),
                )
                .arg(
                    Arg::with_name("enable_non_transferable")
                        .long("enable-non-transferable")
                        .alias("enable-nontransferable")
                        .takes_value(false)
                        .help(
                            "Permanently force tokens to be non-transferable. They may still be burned."
                        ),
                )
                .arg(
                    Arg::with_name("default_account_state")
                        .long("default-account-state")
                        .requires("enable_freeze")
                        .takes_value(true)
                        .possible_values(["initialized", "frozen"])
                        .help("Specify that accounts have a default state. \
                            Note: specifying \"initialized\" adds an extension, which gives \
                            the option of specifying default frozen accounts in the future. \
                            This behavior is not the same as the default, which makes it \
                            impossible to specify a default account state in the future."),
                )
                .arg(
                    Arg::with_name("transfer_fee")
                        .long("transfer-fee")
                        .value_names(&["FEE_IN_BASIS_POINTS", "MAXIMUM_FEE"])
                        .takes_value(true)
                        .number_of_values(2)
                        .hidden(true)
                        .conflicts_with("transfer_fee_basis_points")
                        .conflicts_with("transfer_fee_maximum_fee")
                        .help(
                            "Add a transfer fee to the mint. \
                            The mint authority can set the fee and withdraw collected fees.",
                        ),
                )
                .arg(
                    Arg::with_name("transfer_fee_basis_points")
                        .long("transfer-fee-basis-points")
                        .value_names(&["FEE_IN_BASIS_POINTS"])
                        .takes_value(true)
                        .number_of_values(1)
                        .conflicts_with("transfer_fee")
                        .requires("transfer_fee_maximum_fee")
                        .value_parser(clap::value_parser!(u16))
                        .help(
                            "Add transfer fee to the mint. \
                            The mint authority can set the fee.",
                        ),
                )
                .arg(
                    Arg::with_name("transfer_fee_maximum_fee")
                        .long("transfer-fee-maximum-fee")
                        .value_names(&["MAXIMUM_FEE"])
                        .takes_value(true)
                        .number_of_values(1)
                        .conflicts_with("transfer_fee")
                        .requires("transfer_fee_basis_points")
                        .value_parser(Amount::parse)
                        .help(
                            "Add a UI amount maximum transfer fee to the mint. \
                            The mint authority can set and collect fees"
                        )
                )
                .arg(
                    Arg::with_name("enable_permanent_delegate")
                        .long("enable-permanent-delegate")
                        .takes_value(false)
                        .help(
                            "Enable the mint authority to be permanent delegate for this mint"
                        ),
                )
                .arg(
                    Arg::with_name("enable_confidential_transfers")
                        .long("enable-confidential-transfers")
                        .value_names(&["APPROVE-POLICY"])
                        .takes_value(true)
                        .possible_values(["auto", "manual"])
                        .help(
                            "Enable accounts to make confidential transfers. If \"auto\" \
                            is selected, then accounts are automatically approved to make \
                            confidential transfers. If \"manual\" is selected, then the \
                            confidential transfer mint authority must approve each account \
                            before it can make confidential transfers."
                        )
                )
                .arg(
                    Arg::with_name("transfer_hook")
                        .long("transfer-hook")
                        .value_name("TRANSFER_HOOK_PROGRAM_ID")
                        .validator(|s| is_valid_pubkey(s))
                        .takes_value(true)
                        .help("Enable the mint authority to set the transfer hook program for this mint"),
                )
                .arg(
                    Arg::with_name("enable_metadata")
                        .long("enable-metadata")
                        .conflicts_with("metadata_address")
                        .takes_value(false)
                        .help("Enables metadata in the mint. The mint authority must initialize the metadata."),
                )
                .arg(
                    Arg::with_name("enable_group")
                        .long("enable-group")
                        .conflicts_with("group_address")
                        .takes_value(false)
                        .help("Enables group configurations in the mint. The mint authority must initialize the group."),
                )
                .arg(
                    Arg::with_name("enable_member")
                        .long("enable-member")
                        .conflicts_with("member_address")
                        .takes_value(false)
                        .help("Enables group member configurations in the mint. The mint authority must initialize the member."),
                )
                .arg(
                    Arg::with_name("enable_confidential_mint_burn")
                        .long("enable-confidential-mint-burn")
                        .takes_value(false)
                        .help(
                            "Enables minting of new tokens into confidential balance and burning of tokens directly from the confidential balance"
                        ),
                )
                .arg(
                    Arg::with_name("auditor_pubkey")
                        .long("auditor-pubkey")
                        .value_name("AUDITOR_PUBKEY")
                        .takes_value(true)
                        .help(
                            "The auditor encryption public key for mints with the confidential \
                            transfer extension enabled. The corresponding private key for \
                            this auditor public key can be used to decrypt all confidential \
                            transfers involving tokens from this mint. Currently, the auditor \
                            public key can only be specified as a direct *base64* encoding of \
                            an ElGamal public key. More methods of specifying the auditor public \
                            key will be supported in a future version. To disable auditability \
                            feature for the token, use \"none\"."
                        )
                )
                .arg(
                    Arg::with_name("confidential_supply_pubkey")
                        .long("confidential-supply-pubkey")
                        .value_name("CONFIDENTIAL_SUPPLY_PUBKEY")
                        .takes_value(true)
                        .help(
                            "The confidential supply encryption public key for mints with the \
                            confidential transfer and confidential mint-burn extension enabled. \
                            The corresponding private key for this supply public key can be \
                            used  to decrypt the confidential supply of the token."
                        )
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .arg(memo_arg())
        )
        .subcommand(
            SubCommand::with_name(CommandName::SetInterestRate.into())
                .about("Set the interest rate for an interest-bearing token")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
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
                    .validator(|s| is_valid_signer(s))
                    .value_name("SIGNER")
                    .takes_value(true)
                    .help(
                        "Specify the rate authority keypair. \
                        Defaults to the client keypair address."
                    )
                )
        )
        .subcommand(
            SubCommand::with_name(CommandName::SetTransferHook.into())
                .about("Set the transfer hook program id for a token")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .help("The token address with an existing transfer hook"),
                )
                .arg(
                    Arg::with_name("new_program_id")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("NEW_PROGRAM_ID")
                        .takes_value(true)
                        .required_unless("disable")
                        .index(2)
                        .help("The new transfer hook program id to set for this mint"),
                )
                .arg(
                    Arg::with_name("disable")
                        .long("disable")
                        .takes_value(false)
                        .conflicts_with("new_program_id")
                        .help("Disable transfer hook functionality by setting the program id to None.")
                )
                .arg(
                    Arg::with_name("authority")
                        .long("authority")
                        .alias("owner")
                        .validator(|s| is_valid_signer(s))
                        .value_name("SIGNER")
                        .takes_value(true)
                        .help("Specify the authority keypair. Defaults to the client keypair address.")
                )
        )
        .subcommand(
            SubCommand::with_name(CommandName::InitializeMetadata.into())
                .about("Initialize metadata extension on a token mint")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .help("The token address with no metadata present"),
                )
                .arg(
                    Arg::with_name("name")
                        .value_name("TOKEN_NAME")
                        .takes_value(true)
                        .required(true)
                        .index(2)
                        .help("The name of the token to set in metadata"),
                )
                .arg(
                    Arg::with_name("symbol")
                        .value_name("TOKEN_SYMBOL")
                        .takes_value(true)
                        .required(true)
                        .index(3)
                        .help("The symbol of the token to set in metadata"),
                )
                .arg(
                    Arg::with_name("uri")
                        .value_name("TOKEN_URI")
                        .takes_value(true)
                        .required(true)
                        .index(4)
                        .help("The URI of the token to set in metadata"),
                )
                .arg(
                    Arg::with_name("mint_authority")
                        .long("mint-authority")
                        .alias("owner")
                        .value_name("KEYPAIR")
                        .validator(|s| is_valid_signer(s))
                        .takes_value(true)
                        .help(
                            "Specify the mint authority keypair. \
                             This may be a keypair file or the ASK keyword. \
                             Defaults to the client keypair."
                        ),
                )
                .arg(
                    Arg::with_name("update_authority")
                        .long("update-authority")
                        .value_name("ADDRESS")
                        .validator(|s| is_valid_pubkey(s))
                        .takes_value(true)
                        .help(
                            "Specify the update authority address. \
                             Defaults to the client keypair address."
                        ),
                )
        )
        .subcommand(
            SubCommand::with_name(CommandName::UpdateMetadata.into())
                .about("Update metadata on a token mint that has the extension")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .help("The token address with no metadata present"),
                )
                .arg(
                    Arg::with_name("field")
                        .value_name("FIELD_NAME")
                        .takes_value(true)
                        .required(true)
                        .index(2)
                        .help("The name of the field to update. Can be a base field (\"name\", \"symbol\", or \"uri\") or any new field to add."),
                )
                .arg(
                    Arg::with_name("value")
                        .value_name("VALUE_STRING")
                        .takes_value(true)
                        .index(3)
                        .required_unless("remove")
                        .help("The value for the field"),
                )
                .arg(
                    Arg::with_name("remove")
                        .long("remove")
                        .takes_value(false)
                        .conflicts_with("value")
                        .help("Remove the key and value for the given field. Does not work with base fields: \"name\", \"symbol\", or \"uri\".")
                )
                .arg(
                    Arg::with_name("authority")
                    .long("authority")
                    .validator(|s| is_valid_signer(s))
                    .value_name("SIGNER")
                    .takes_value(true)
                    .help("Specify the metadata update authority keypair. Defaults to the client keypair.")
                )
                .nonce_args(true)
                .arg(transfer_lamports_arg())
                .offline_args_config(&SignOnlyNeedsTransferLamports{}),
        )
        .subcommand(
            SubCommand::with_name(CommandName::InitializeGroup.into())
                .about("Initialize group extension on a token mint")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .help("The token address of the group account."),
                )
                .arg(
                        Arg::with_name("max_size")
                        .value_parser(clap::value_parser!(u64))
                        .value_name("MAX_SIZE")
                        .takes_value(true)
                        .required(true)
                        .index(2)
                        .help("The number of members in the group."),
                    )
                .arg(
                    Arg::with_name("mint_authority")
                        .long("mint-authority")
                        .alias("owner")
                        .value_name("KEYPAIR")
                        .validator(|s| is_valid_signer(s))
                        .takes_value(true)
                        .help(
                            "Specify the mint authority keypair. \
                             This may be a keypair file or the ASK keyword. \
                             Defaults to the client keypair."
                        ),
                )
                .arg(
                    Arg::with_name("update_authority")
                        .long("update-authority")
                        .value_name("ADDRESS")
                        .validator(|s| is_valid_pubkey(s))
                        .takes_value(true)
                        .help(
                            "Specify the update authority address. \
                             Defaults to the client keypair address."
                        ),
                )
        )
        .subcommand(
            SubCommand::with_name(CommandName::UpdateGroupMaxSize.into())
                .about("Updates the maximum number of members for a group.")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .help("The token address of the group account."),
                )
                .arg(
                        Arg::with_name("new_max_size")
                        .value_parser(clap::value_parser!(u64))
                        .value_name("NEW_MAX_SIZE")
                        .takes_value(true)
                        .required(true)
                        .index(2)
                        .help("The number of members in the group."),
                    )
                .arg(
                    Arg::with_name("update_authority")
                        .long("update-authority")
                        .value_name("SIGNER")
                        .validator(|s| is_valid_signer(s))
                        .takes_value(true)
                        .help(
                            "Specify the update authority address. \
                             Defaults to the client keypair address."
                        ),
                )
        )
        .subcommand(
            SubCommand::with_name(CommandName::InitializeMember.into())
                .about("Initialize group member extension on a token mint")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .index(1)
                        .help("The token address of the member account."),
                )
                .arg(
                    Arg::with_name("group_token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("GROUP_TOKEN_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .index(2)
                        .help("The token address of the group account that the token will join."),
                )
                .arg(
                    Arg::with_name("mint_authority")
                        .long("mint-authority")
                        .alias("owner")
                        .value_name("KEYPAIR")
                        .validator(|s| is_valid_signer(s))
                        .takes_value(true)
                        .help(
                            "Specify the mint authority keypair. \
                             This may be a keypair file or the ASK keyword. \
                             Defaults to the client keypair."
                        ),
                )
                .arg(
                    Arg::with_name("group_update_authority")
                        .long("group-update-authority")
                        .value_name("KEYPAIR")
                        .validator(|s| is_valid_signer(s))
                        .takes_value(true)
                        .help(
                            "Specify the update authority keypair. \
                             This may be a keypair file or the ASK keyword. \
                             Defaults to the client keypair address."
                        ),
                )
        )
        .subcommand(
            SubCommand::with_name(CommandName::CreateAccount.into())
                .about("Create a new token account")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token that the account will hold"),
                )
                .arg(
                    Arg::with_name("account_keypair")
                        .value_name("ACCOUNT_KEYPAIR")
                        .validator(|s| is_valid_signer(s))
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
                        .validator(|s| is_valid_pubkey(s))
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .min_values(MIN_SIGNERS)
                        .max_values(MAX_SIGNERS)
                        .help(multisig_member_help),
                )
                .arg(
                    Arg::with_name("address_keypair")
                        .long("address-keypair")
                        .value_name("ADDRESS_KEYPAIR")
                        .validator(|s| is_valid_signer(s))
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
                        .validator(|s| is_valid_pubkey(s))
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
                        .possible_values(CliAuthorityType::iter().map(Into::<&str>::into).collect::<Vec<_>>())
                        .index(2)
                        .required(true)
                        .help("The new authority type. \
                            Token mints support `mint`, `freeze`, and mint extension authorities; \
                            Token accounts support `owner`, `close`, and account extension \
                            authorities."),
                )
                .arg(
                    Arg::with_name("new_authority")
                        .validator(|s| is_valid_pubkey(s))
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
                        .validator(|s| is_valid_signer(s))
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
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("Token to transfer"),
                )
                .arg(
                    Arg::with_name("amount")
                        .value_parser(Amount::parse)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to send, in tokens; accepts keyword ALL"),
                )
                .arg(
                    Arg::with_name("recipient")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("RECIPIENT_WALLET_ADDRESS or RECIPIENT_TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(3)
                        .required(true)
                        .help("If a token account address is provided, use it as the recipient. \
                               Otherwise assume the recipient address is a user wallet and transfer to \
                               the associated token account")
                )
                .arg(
                    Arg::with_name("from")
                        .validator(|s| is_valid_pubkey(s))
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
                        .conflicts_with("confidential")
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
                    Arg::with_name("no_recipient_is_ata_owner")
                        .long("no-recipient-is-ata-owner")
                        .takes_value(false)
                        .requires("sign_only")
                        .help("In sign-only mode, specifies that the recipient is the owner of the associated token account rather than an actual token account"),
                )
                .arg(
                    Arg::with_name("recipient_is_ata_owner")
                        .long("recipient-is-ata-owner")
                        .takes_value(false)
                        .hidden(true)
                        .conflicts_with("no_recipient_is_ata_owner")
                        .requires("sign_only")
                        .help("recipient-is-ata-owner is now the default behavior. The option has been deprecated and will be removed in a future release."),
                )
                .arg(
                    Arg::with_name("expected_fee")
                        .long("expected-fee")
                        .value_parser(Amount::parse)
                        .value_name("EXPECTED_FEE")
                        .takes_value(true)
                        .help("Expected fee amount collected during the transfer"),
                )
                .arg(
                    Arg::with_name("transfer_hook_account")
                        .long("transfer-hook-account")
                        .validator(|s| validate_transfer_hook_account(s))
                        .value_name("PUBKEY:ROLE")
                        .takes_value(true)
                        .multiple(true)
                        .min_values(0_usize)
                        .help("Additional pubkey(s) required for a transfer hook and their \
                            role, in the format \"<PUBKEY>:<ROLE>\". The role must be \
                            \"readonly\", \"writable\". \"readonly-signer\", or \"writable-signer\".\
                            Used for offline transaction creation and signing.")
                )
                .arg(
                    Arg::with_name("confidential")
                        .long("confidential")
                        .takes_value(false)
                        .conflicts_with("fund_recipient")
                        .help("Send tokens confidentially. Both sender and recipient accounts must \
                            be pre-configured for confidential transfers.")
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
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token account address to burn from"),
                )
                .arg(
                    Arg::with_name("amount")
                        .value_parser(Amount::parse)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to burn, in tokens; accepts keyword ALL"),
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
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token to mint"),
                )
                .arg(
                    Arg::with_name("amount")
                        .value_parser(Amount::parse)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to mint, in tokens"),
                )
                .arg(
                    Arg::with_name("recipient")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("RECIPIENT_TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .conflicts_with("recipient_owner")
                        .index(3)
                        .help("The token account address of recipient \
                            [default: associated token account for --mint-authority]"),
                )
                .arg(
                    Arg::with_name("recipient_owner")
                        .long("recipient-owner")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("RECIPIENT_WALLET_ADDRESS")
                        .takes_value(true)
                        .conflicts_with("recipient")
                        .help("The owner of the recipient associated token account"),
                )
                .arg(
                    Arg::with_name("mint_authority")
                        .long("mint-authority")
                        .alias("owner")
                        .value_name("KEYPAIR")
                        .validator(|s| is_valid_signer(s))
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
                        .validator(|s| is_valid_pubkey(s))
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
                        .validator(|s| is_valid_signer(s))
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
                        .validator(|s| is_valid_pubkey(s))
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
                        .validator(|s| is_valid_signer(s))
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
                        .value_parser(Amount::parse)
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
                        .validator(|s| is_valid_signer(s))
                        .takes_value(true)
                        .index(2)
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
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .help("The address of the auxiliary token account to unwrap \
                            [default: associated token account for --owner]"),
                )
                .arg(
                    Arg::with_name("wallet_keypair")
                        .value_name("KEYPAIR")
                        .validator(|s| is_valid_signer(s))
                        .takes_value(true)
                        .index(2)
                        .help(
                            "Specify the keypair for the wallet which owns the wrapped SOL. \
                             This wallet will receive the unwrapped SOL. \
                             This may be a keypair file or the ASK keyword. \
                             Defaults to the client keypair."
                        ),
                )
                .arg(owner_address_arg())
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Approve.into())
                .about("Approve a delegate for a token account")
                .arg(
                    Arg::with_name("account")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to delegate"),
                )
                .arg(
                    Arg::with_name("amount")
                        .value_parser(Amount::parse)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to approve, in tokens"),
                )
                .arg(
                    Arg::with_name("delegate")
                        .validator(|s| is_valid_pubkey(s))
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
                        .validator(|s| is_valid_pubkey(s))
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
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required_unless("address")
                        .help("Token of the associated account to close. \
                              To close a specific account, use the `--address` parameter instead"),
                )
                .arg(
                    Arg::with_name("recipient")
                        .long("recipient")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("REFUND_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .help("The address of the account to receive remaining SOL [default: --owner]"),
                )
                .arg(
                    Arg::with_name("close_authority")
                        .long("close-authority")
                        .value_name("KEYPAIR")
                        .validator(|s| is_valid_signer(s))
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
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .conflicts_with("token")
                        .help("Specify the token account to close \
                            [default: owner's associated token account]"),
                )
                .arg(owner_address_arg())
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name(CommandName::CloseMint.into())
                .about("Close a token mint")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("Token to close"),
                )
                .arg(
                    Arg::with_name("recipient")
                        .long("recipient")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("REFUND_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .help("The address of the account to receive remaining SOL [default: --owner]"),
                )
                .arg(
                    Arg::with_name("close_authority")
                        .long("close-authority")
                        .value_name("KEYPAIR")
                        .validator(|s| is_valid_signer(s))
                        .takes_value(true)
                        .help(
                            "Specify the token's close authority. \
                            This may be a keypair file or the ASK keyword. \
                            Defaults to the client keypair.",
                        ),
                )
                .arg(owner_address_arg())
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name(CommandName::Balance.into())
                .about("Get token account balance")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required_unless("address")
                        .help("Token of associated account. To query a specific account, use the `--address` parameter instead"),
                )
                .arg(owner_address_arg().conflicts_with("address"))
                .arg(
                    Arg::with_name("address")
                        .validator(|s| is_valid_pubkey(s))
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
                        .validator(|s| is_valid_pubkey(s))
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
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .help("Limit results to the given token. [Default: list accounts for all tokens]"),
                )
                .arg(
                    Arg::with_name("delegated")
                        .long("delegated")
                        .takes_value(false)
                        .conflicts_with("externally_closeable")
                        .help(
                            "Limit results to accounts with transfer delegations"
                        ),
                )
                .arg(
                    Arg::with_name("externally_closeable")
                        .long("externally-closeable")
                        .takes_value(false)
                        .conflicts_with("delegated")
                        .help(
                            "Limit results to accounts with external close authorities"
                        ),
                )
                .arg(
                    Arg::with_name("addresses_only")
                        .long("addresses-only")
                        .takes_value(false)
                        .conflicts_with("verbose")
                        .conflicts_with("output_format")
                        .help(
                            "Print token account addresses only"
                        ),
                )
                .arg(owner_address_arg())
        )
        .subcommand(
            SubCommand::with_name(CommandName::Address.into())
                .about("Get wallet address")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
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
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .conflicts_with("address")
                        .required_unless("address")
                        .help("Token of associated account. \
                               To query a specific account, use the `--address` parameter instead"),
                )
                .arg(
                    Arg::with_name(OWNER_ADDRESS_ARG.name)
                        .takes_value(true)
                        .value_name("OWNER_ADDRESS")
                        .validator(|s| is_valid_signer(s))
                        .help(OWNER_ADDRESS_ARG.help)
                        .index(2)
                        .conflicts_with("address")
                        .help("Owner of the associated account for the specified token. \
                               To query a specific account, use the `--address` parameter instead. \
                               Defaults to the client keypair."),
                )
                .arg(
                    Arg::with_name("address")
                        .validator(|s| is_valid_pubkey(s))
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
                    .validator(|s| is_valid_pubkey(s))
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
                    .validator(|s| is_valid_pubkey(s))
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
                        .validator(|s| is_valid_pubkey(s))
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
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to require transfer memos for")
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
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to stop requiring transfer memos for"),
                )
                .arg(
                    owner_address_arg()
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::EnableCpiGuard.into())
                .about("Enable CPI Guard for token account")
                .arg(
                    Arg::with_name("account")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to enable CPI Guard for")
                )
                .arg(
                    owner_address_arg()
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::DisableCpiGuard.into())
                .about("Disable CPI Guard for token account")
                .arg(
                    Arg::with_name("account")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to disable CPI Guard for"),
                )
                .arg(
                    owner_address_arg()
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::UpdateDefaultAccountState.into())
                .about("Updates default account state for the mint. Requires the default account state extension.")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token mint to update default account state"),
                )
                .arg(
                    Arg::with_name("state")
                        .value_name("STATE")
                        .takes_value(true)
                        .possible_values(["initialized", "frozen"])
                        .index(2)
                        .required(true)
                        .help("The new default account state."),
                )
                .arg(
                    Arg::with_name("freeze_authority")
                        .long("freeze-authority")
                        .value_name("KEYPAIR")
                        .validator(|s| is_valid_signer(s))
                        .takes_value(true)
                        .help(
                            "Specify the token's freeze authority. \
                            This may be a keypair file or the ASK keyword. \
                            Defaults to the client keypair.",
                        ),
                )
                .arg(owner_address_arg())
                .arg(multisig_signer_arg())
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name(CommandName::UpdateMetadataAddress.into())
                .about("Updates metadata pointer address for the mint. Requires the metadata pointer extension.")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token mint to update the metadata pointer address"),
                )
                .arg(
                    Arg::with_name("metadata_address")
                        .index(2)
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("METADATA_ADDRESS")
                        .takes_value(true)
                        .required_unless("disable")
                        .help("Specify address that stores token's metadata-pointer"),
                )
                .arg(
                    Arg::with_name("disable")
                        .long("disable")
                        .takes_value(false)
                        .conflicts_with("metadata_address")
                        .help("Unset metadata pointer address.")
                )
                .arg(
                    Arg::with_name("authority")
                        .long("authority")
                        .value_name("KEYPAIR")
                        .validator(|s| is_valid_signer(s))
                        .takes_value(true)
                        .help(
                            "Specify the token's metadata-pointer authority. \
                            This may be a keypair file or the ASK keyword. \
                            Defaults to the client keypair.",
                        ),
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::UpdateGroupAddress.into())
                .about("Updates group pointer address for the mint. Requires the group pointer extension.")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token mint to update the group pointer address"),
                )
                .arg(
                    Arg::with_name("group_address")
                        .index(2)
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("GROUP_ADDRESS")
                        .takes_value(true)
                        .required_unless("disable")
                        .help("Specify address that stores token's group-pointer"),
                )
                .arg(
                    Arg::with_name("disable")
                        .long("disable")
                        .takes_value(false)
                        .conflicts_with("group_address")
                        .help("Unset group pointer address.")
                )
                .arg(
                    Arg::with_name("authority")
                        .long("authority")
                        .value_name("KEYPAIR")
                        .validator(|s| is_valid_signer(s))
                        .takes_value(true)
                        .help(
                            "Specify the token's group-pointer authority. \
                            This may be a keypair file or the ASK keyword. \
                            Defaults to the client keypair.",
                        ),
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::UpdateMemberAddress.into())
                .about("Updates group member pointer address for the mint. Requires the group member pointer extension.")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token mint to update the group member pointer address"),
                )
                .arg(
                    Arg::with_name("member_address")
                        .index(2)
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("MEMBER_ADDRESS")
                        .takes_value(true)
                        .required_unless("disable")
                        .help("Specify address that stores token's group-member-pointer"),
                )
                .arg(
                    Arg::with_name("disable")
                        .long("disable")
                        .takes_value(false)
                        .conflicts_with("member_address")
                        .help("Unset group member pointer address.")
                )
                .arg(
                    Arg::with_name("authority")
                        .long("authority")
                        .value_name("KEYPAIR")
                        .validator(|s| is_valid_signer(s))
                        .takes_value(true)
                        .help(
                            "Specify the token's group-member-pointer authority. \
                            This may be a keypair file or the ASK keyword. \
                            Defaults to the client keypair.",
                        ),
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::WithdrawWithheldTokens.into())
                .about("Withdraw withheld transfer fee tokens from mint and / or account(s)")
                .arg(
                    Arg::with_name("account")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token account to receive withdrawn tokens"),
                )
                .arg(
                    Arg::with_name("source")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .multiple(true)
                        .min_values(0_usize)
                        .index(2)
                        .help("The token accounts to withdraw from")
                )
                .arg(
                    Arg::with_name("include_mint")
                        .long("include-mint")
                        .takes_value(false)
                        .help("Also withdraw withheld tokens from the mint"),
                )
                .arg(
                    Arg::with_name("withdraw_withheld_authority")
                        .long("withdraw-withheld-authority")
                        .value_name("KEYPAIR")
                        .validator(|s| is_valid_signer(s))
                        .takes_value(true)
                        .help(
                            "Specify the withdraw withheld authority keypair. \
                             This may be a keypair file or the ASK keyword. \
                             Defaults to the client keypair."
                        ),
                )
                .arg(owner_address_arg())
                .arg(multisig_signer_arg())
                .group(
                    ArgGroup::with_name("source_or_mint")
                        .arg("source")
                        .arg("include_mint")
                        .multiple(true)
                        .required(true)
                )
        )
        .subcommand(
            SubCommand::with_name(CommandName::SetTransferFee.into())
                .about("Set the transfer fee for a token with a configured transfer fee")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("The interest-bearing token address"),
                )
                .arg(
                    Arg::with_name("transfer_fee_basis_points")
                        .value_name("FEE_IN_BASIS_POINTS")
                        .takes_value(true)
                        .required(true)
                        .help("The new transfer fee in basis points"),
                )
                .arg(
                    Arg::with_name("maximum_fee")
                        .value_name("MAXIMUM_FEE")
                        .value_parser(Amount::parse)
                        .takes_value(true)
                        .required(true)
                        .help("The new maximum transfer fee in UI amount"),
                )
                .arg(
                    Arg::with_name("transfer_fee_authority")
                    .long("transfer-fee-authority")
                    .validator(|s| is_valid_signer(s))
                    .value_name("SIGNER")
                    .takes_value(true)
                    .help(
                        "Specify the rate authority keypair. \
                        Defaults to the client keypair address."
                    )
                )
                .arg(mint_decimals_arg())
                .offline_args_config(&SignOnlyNeedsMintDecimals{})
        )
        .subcommand(
            SubCommand::with_name(CommandName::WithdrawExcessLamports.into())
                .about("Withdraw lamports from a Token Program owned account")
                .arg(
                    Arg::with_name("from")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("SOURCE_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Specify the address of the account to recover lamports from"),
                )
                .arg(
                    Arg::with_name("recipient")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("REFUND_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .required(true)
                        .help("Specify the address of the account to send lamports to"),
                )
                .arg(owner_address_arg())
                .arg(multisig_signer_arg())
        )
        .subcommand(
            SubCommand::with_name(CommandName::UpdateConfidentialTransferSettings.into())
                .about("Update confidential transfer configuration for a token")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The address of the token mint to update confidential transfer configuration for")
                )
                .arg(
                    Arg::with_name("approve_policy")
                        .long("approve-policy")
                        .value_name("APPROVE_POLICY")
                        .takes_value(true)
                        .possible_values(["auto", "manual"])
                        .help(
                            "Policy for enabling accounts to make confidential transfers. If \"auto\" \
                            is selected, then accounts are automatically approved to make \
                            confidential transfers. If \"manual\" is selected, then the \
                            confidential transfer mint authority must approve each account \
                            before it can make confidential transfers."
                        )
                )
                .arg(
                    Arg::with_name("auditor_pubkey")
                        .long("auditor-pubkey")
                        .value_name("AUDITOR_PUBKEY")
                        .takes_value(true)
                        .help(
                            "The auditor encryption public key. The corresponding private key for \
                            this auditor public key can be used to decrypt all confidential \
                            transfers involving tokens from this mint. Currently, the auditor \
                            public key can only be specified as a direct *base64* encoding of \
                            an ElGamal public key. More methods of specifying the auditor public \
                            key will be supported in a future version. To disable auditability \
                            feature for the token, use \"none\"."
                        )
                )
                .group(
                    ArgGroup::with_name("update_fields").args(&["approve_policy", "auditor_pubkey"])
                        .required(true)
                        .multiple(true)
                )
                .arg(
                    Arg::with_name("confidential_transfer_authority")
                        .long("confidential-transfer-authority")
                        .validator(|s| is_valid_signer(s))
                        .value_name("SIGNER")
                        .takes_value(true)
                        .help(
                            "Specify the confidential transfer authority keypair. \
                            Defaults to the client keypair address."
                        )
                )
                .nonce_args(true)
                .offline_args(),
        )
        .subcommand(
            SubCommand::with_name(CommandName::ConfigureConfidentialTransferAccount.into())
                .about("Configure confidential transfers for token account")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required_unless("address")
                        .help("The token address with confidential transfers enabled"),
                )
                .arg(
                    Arg::with_name("address")
                        .long("address")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .conflicts_with("token")
                        .help("The address of the token account to configure confidential transfers for \
                            [default: owner's associated token account]")
                )
                .arg(
                    owner_address_arg()
                )
                .arg(
                    Arg::with_name("maximum_pending_balance_credit_counter")
                        .long("maximum-pending-balance-credit-counter")
                        .value_name("MAXIMUM-CREDIT-COUNTER")
                        .takes_value(true)
                        .help(
                            "The maximum pending balance credit counter. \
                            This parameter limits the number of confidential transfers that a token account \
                            can receive to facilitate decryption of the encrypted balance. \
                            Defaults to 65536 (2^16)"
                        )
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::EnableConfidentialCredits.into())
                .about("Enable confidential transfers for token account. To enable confidential transfers \
                for the first time, use `configure-confidential-transfer-account` instead.")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required_unless("address")
                        .help("The token address with confidential transfers enabled"),
                )
                .arg(
                    Arg::with_name("address")
                        .long("address")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .conflicts_with("token")
                        .help("The address of the token account to enable confidential transfers for \
                            [default: owner's associated token account]")
                )
                .arg(
                    owner_address_arg()
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::DisableConfidentialCredits.into())
                .about("Disable confidential transfers for token account")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required_unless("address")
                        .help("The token address with confidential transfers enabled"),
                )
                .arg(
                    Arg::with_name("address")
                        .long("address")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .conflicts_with("token")
                        .help("The address of the token account to disable confidential transfers for \
                            [default: owner's associated token account]")
                )
                .arg(
                    owner_address_arg()
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::EnableNonConfidentialCredits.into())
                .about("Enable non-confidential transfers for token account.")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required_unless("address")
                        .help("The token address with confidential transfers enabled"),
                )
                .arg(
                    Arg::with_name("address")
                        .long("address")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .conflicts_with("token")
                        .help("The address of the token account to enable non-confidential transfers for \
                            [default: owner's associated token account]")
                )
                .arg(
                    owner_address_arg()
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::DisableNonConfidentialCredits.into())
                .about("Disable non-confidential transfers for token account")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required_unless("address")
                        .help("The token address with confidential transfers enabled"),
                )
                .arg(
                    Arg::with_name("address")
                        .long("address")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .conflicts_with("token")
                        .help("The address of the token account to disable non-confidential transfers for \
                            [default: owner's associated token account]")
                )
                .arg(
                    owner_address_arg()
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::DepositConfidentialTokens.into())
                .about("Deposit amounts for confidential transfers")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token address with confidential transfers enabled"),
                )
                .arg(
                    Arg::with_name("amount")
                        .value_parser(Amount::parse)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to deposit; accepts keyword ALL"),
                )
                .arg(
                    Arg::with_name("address")
                        .long("address")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .help("The address of the token account to configure confidential transfers for \
                            [default: owner's associated token account]")
                )
                .arg(
                    owner_address_arg()
                )
                .arg(multisig_signer_arg())
                .arg(mint_decimals_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::WithdrawConfidentialTokens.into())
                .about("Withdraw amounts for confidential transfers")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token address with confidential transfers enabled"),
                )
                .arg(
                    Arg::with_name("amount")
                        .value_parser(Amount::parse)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to deposit; accepts keyword ALL"),
                )
                .arg(
                    Arg::with_name("address")
                        .long("address")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .help("The address of the token account to configure confidential transfers for \
                            [default: owner's associated token account]")
                )
                .arg(
                    owner_address_arg()
                )
                .arg(multisig_signer_arg())
                .arg(mint_decimals_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::ApplyPendingBalance.into())
                .about("Collect confidential tokens from pending to available balance")
                .arg(
                    Arg::with_name("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required_unless("address")
                        .help("The token address with confidential transfers enabled"),
                )
                .arg(
                    Arg::with_name("address")
                        .long("address")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .help("The address of the token account to configure confidential transfers for \
                            [default: owner's associated token account]")
                )
                .arg(
                    owner_address_arg()
                )
                .arg(multisig_signer_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::MintConfidentialTokens.into())
                .about("Mint tokens amounts for into confidential balance")
                .arg(
                    Arg::with_name("token")
                        .long("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token address with confidential transfers enabled"),
                )
                .arg(
                    Arg::with_name("amount")
                        .value_parser(Amount::parse)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to deposit; accepts keyword ALL"),
                )
                .arg(
                    Arg::with_name("address")
                        .long("address")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .help("The address of the token account to configure confidential transfers for \
                            [default: owner's associated token account]")
                )
                .arg(
                    owner_address_arg()
                )
                .arg(multisig_signer_arg())
                .arg(mint_decimals_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::BurnConfidentialTokens.into())
                .about("Burn tokens from available confidential balance")
                .arg(
                    Arg::with_name("token")
                        .long("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token address with confidential transfers enabled"),
                )
                .arg(
                    Arg::with_name("amount")
                        .value_parser(Amount::parse)
                        .value_name("TOKEN_AMOUNT")
                        .takes_value(true)
                        .index(2)
                        .required(true)
                        .help("Amount to deposit; accepts keyword ALL"),
                )
                .arg(
                    Arg::with_name("address")
                        .long("address")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .help("The address of the token account to configure confidential transfers for \
                            [default: owner's associated token account]")
                )
                .arg(
                    owner_address_arg()
                )
                .arg(multisig_signer_arg())
                .arg(mint_decimals_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::ConfidentialBalance.into())
                .about("Display confidential balance")
                .arg(
                    Arg::with_name("token")
                        .long("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token address with confidential transfers enabled"),
                )
                .arg(
                    Arg::with_name("address")
                        .long("address")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_ACCOUNT_ADDRESS")
                        .takes_value(true)
                        .index(2)
                        .help("The address of the token account to for which to fetch the confidential balance")
                )
                .arg(
                    Arg::with_name("authority")
                        .long("authority")
                        .alias("owner")
                        .validator(|s| is_valid_signer(s))
                        .value_name("SIGNER")
                        .takes_value(true)
                        .help("Keypair from which encryption keys for token account were derived.")
                )
                .arg(
                    owner_address_arg()
                )
                .arg(multisig_signer_arg())
                .arg(mint_decimals_arg())
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::ConfidentialSupply.into())
                .about("Display supply of confidential token")
                .arg(
                    Arg::with_name("token")
                        .long("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token address with confidential transfers enabled"),
                )
                .arg(
                    Arg::with_name("authority")
                        .long("authority")
                        .alias("owner")
                        .validator(|s| is_valid_signer(s))
                        .value_name("SIGNER")
                        .takes_value(true)
                        .help("Keypair from which the supply elgamal keypair is derived. \
                              Either the authority or the confidential-supply-keypair have \
                              to be specified in order for the supply to be decrypted.")
                )
                .arg(
                    Arg::with_name("confidential_supply_keypair")
                        .long("confidential-supply-keypair")
                        .value_name("CONFIDENTIAL_SUPPLY_KEYPAIR")
                        .takes_value(true)
                        .help(
                            "The confidential supply encryption keypair used to decrypt ElGamalCiphertext supply. \
                              Either the authority or the confidential-supply-keypair have \
                              to be specified in order for the supply to be decrypted."
                        )
                )
                .arg(
                    Arg::with_name("confidential_supply_aes_key")
                        .long("confidential-supply-aes-key")
                        .value_name("CONFIDENTIAL_SUPPLY_AES_KEY")
                        .takes_value(true)
                        .help(
                            "The aes key used to decrypt the decryptable portion of the confidential supply."
                        )
                )
                .nonce_args(true)
        )
        .subcommand(
            SubCommand::with_name(CommandName::RotateSupplyElgamal.into())
                .about("Display supply of confidential token")
                .arg(
                    Arg::with_name("token")
                        .long("token")
                        .validator(|s| is_valid_pubkey(s))
                        .value_name("TOKEN_MINT_ADDRESS")
                        .takes_value(true)
                        .index(1)
                        .required(true)
                        .help("The token address with confidential transfers enabled"),
                )
                .arg(
                    Arg::with_name("authority")
                        .long("authority")
                        .alias("owner")
                        .validator(|s| is_valid_signer(s))
                        .value_name("SIGNER")
                        .takes_value(true)
                        .required(true)
                        .help("Keypair holding the authority over the confidential-mint-burn extension.")
                )
                .arg(
                    Arg::with_name("current_supply_keypair")
                        .long("current-supply-keypair")
                        .value_name("CURRENT_SUPPLY_KEYPAIR")
                        .takes_value(true)
                        .required(true)
                        .help(
                            "The current confidential supply encryption keypair."
                        )
                )
                .arg(
                    Arg::with_name("supply_aes_key")
                        .long("supply-aes-key")
                        .value_name("SUPPLY_AES_KEY")
                        .takes_value(true)
                        .required(true)
                        .help(
                            "The aes key to decrypt the decryptable confidential supply."
                        )
                )
                .arg(
                    Arg::with_name("new_supply_keypair")
                        .long("new-supply-keypair")
                        .value_name("NEW_SUPPLY_KEYPAIR")
                        .takes_value(true)
                        .required(true)
                        .help(
                            "The new confidential supply encryption keypair to rotate to."
                        )
                )
                .nonce_args(true)
        )
}
