use {
    crate::config::Error,
    clap::{
        builder::{PossibleValuesParser, TypedValueParser},
        ArgGroup, ArgMatches, Args, Parser, Subcommand,
    },
    solana_clap_v3_utils::{
        input_parsers::{parse_url_or_moniker, Amount},
        input_validators::{is_valid_pubkey, is_valid_signer},
        keypair::{pubkey_from_path, signer_from_path},
    },
    solana_cli_output::OutputFormat,
    solana_remote_wallet::remote_wallet::RemoteWalletManager,
    solana_sdk::{pubkey::Pubkey, signer::Signer},
    spl_single_pool::{self, find_pool_address},
    std::{rc::Rc, str::FromStr, sync::Arc},
};

#[derive(Clone, Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// Configuration file to use
    #[clap(global(true), short = 'C', long = "config", id = "PATH")]
    pub config_file: Option<String>,

    /// Show additional information
    #[clap(global(true), short, long)]
    pub verbose: bool,

    /// Simulate transaction instead of executing
    #[clap(global(true), long, alias = "dryrun")]
    pub dry_run: bool,

    /// URL for Solana's JSON RPC or moniker (or their first letter):
    /// [mainnet-beta, testnet, devnet, localhost].
    /// Default from the configuration file.
    #[clap(
        global(true),
        short = 'u',
        long = "url",
        id = "URL_OR_MONIKER",
        value_parser = parse_url_or_moniker,
    )]
    pub json_rpc_url: Option<String>,

    /// Specify the fee-payer account. This may be a keypair file, the ASK
    /// keyword or the pubkey of an offline signer, provided an appropriate
    /// --signer argument is also passed. Defaults to the client keypair.
    #[clap(
        global(true),
        long,
        id = "PAYER_KEYPAIR",
        validator = |s| is_valid_signer(s),
    )]
    pub fee_payer: Option<SignerArg>,

    /// Return information in specified output format
    #[clap(
        global(true),
        long = "output",
        id = "FORMAT",
        conflicts_with = "verbose",
        value_parser = PossibleValuesParser::new(["json", "json-compact"]).map(|o| parse_output_format(&o)),
    )]
    pub output_format: Option<OutputFormat>,

    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Clone, Debug, Subcommand)]
pub enum Command {
    /// Commands used to initialize or manage existing single-validator stake
    /// pools. Other than initializing new pools, most users should never
    /// need to use these.
    Manage(ManageCli),

    /// Deposit delegated stake into a pool in exchange for pool tokens, closing
    /// out the original stake account. Provide either a stake account
    /// address, or a pool or vote account address along with the
    /// --default-stake-account flag to use an account created with
    /// create-stake.
    Deposit(DepositCli),

    /// Withdraw stake into a new stake account, burning tokens in exchange.
    /// Provide either pool or vote account address, plus either an amount of
    /// tokens to burn or the ALL keyword to burn all.
    Withdraw(WithdrawCli),

    /// Create and delegate a new stake account to a given validator, using a
    /// default address linked to the intended depository pool
    CreateDefaultStake(CreateStakeCli),

    /// Display info for one or all single-validator stake pool(s)
    Display(DisplayCli),
}

#[derive(Clone, Debug, Parser)]
pub struct ManageCli {
    #[clap(subcommand)]
    pub manage: ManageCommand,
}

#[derive(Clone, Debug, Subcommand)]
pub enum ManageCommand {
    /// Permissionlessly create the single-validator stake pool for a given
    /// validator vote account if one does not already exist. The fee payer
    /// also pays rent-exemption for accounts, along with the
    /// cluster-configured minimum stake delegation
    Initialize(InitializeCli),

    /// Permissionlessly re-stake the pool stake account in the case when it has
    /// been deactivated. This may happen if the validator is
    /// force-deactivated, and then later reactivated using the same address
    /// for its vote account.
    ReactivatePoolStake(ReactivateCli),

    /// Permissionlessly create default MPL token metadata for the pool mint.
    /// Normally this is done automatically upon initialization, so this
    /// does not need to be called.
    CreateTokenMetadata(CreateMetadataCli),

    /// Modify the MPL token metadata associated with the pool mint. This action
    /// can only be performed by the validator vote account's withdraw
    /// authority
    UpdateTokenMetadata(UpdateMetadataCli),
}

#[derive(Clone, Debug, Args)]
pub struct InitializeCli {
    /// The vote account to create the pool for
    #[clap(value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Pubkey,

    /// Do not create MPL metadata for the pool mint
    #[clap(long)]
    pub skip_metadata: bool,
}

#[derive(Clone, Debug, Args)]
#[clap(group(pool_source_group()))]
pub struct ReactivateCli {
    /// The pool to reactivate
    #[clap(short, long = "pool", value_parser = |p: &str| parse_address(p, "pool_address"))]
    pub pool_address: Option<Pubkey>,

    /// The vote account corresponding to the pool to reactivate
    #[clap(long = "vote-account", value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Option<Pubkey>,

    // backdoor for testing, theres no reason to ever use this
    #[clap(long, hide = true)]
    pub skip_deactivation_check: bool,
}

#[derive(Clone, Debug, Args)]
#[clap(group(ArgGroup::new("stake-source").required(true).args(&["stake-account-address", "default-stake-account"])))]
#[clap(group(pool_source_group().required(false)))]
pub struct DepositCli {
    /// The stake account to deposit from. Must be in the same activation state
    /// as the pool's stake account
    #[clap(value_parser = |p: &str| parse_address(p, "stake_account_address"))]
    pub stake_account_address: Option<Pubkey>,

    /// Instead of using a stake account by address, use the user's default
    /// account for a specified pool
    #[clap(
        short,
        long,
        conflicts_with = "stake-account-address",
        requires = "pool-source"
    )]
    pub default_stake_account: bool,

    /// The pool to deposit into. Optional when stake account is provided
    #[clap(short, long = "pool", value_parser = |p: &str| parse_address(p, "pool_address"))]
    pub pool_address: Option<Pubkey>,

    /// The vote account corresponding to the pool to deposit into. Optional
    /// when stake account or pool is provided
    #[clap(long = "vote-account", value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Option<Pubkey>,

    /// Signing authority on the stake account to be deposited. Defaults to the
    /// client keypair
    #[clap(long = "withdraw-authority", id = "STAKE_WITHDRAW_AUTHORITY_KEYPAIR", validator = |s| is_valid_signer(s))]
    pub stake_withdraw_authority: Option<SignerArg>,

    /// The token account to mint to. Defaults to the client keypair's
    /// associated token account
    #[clap(long = "token-account", value_parser = |p: &str| parse_address(p, "token_account_address"))]
    pub token_account_address: Option<Pubkey>,

    /// The wallet to refund stake account rent to. Defaults to the client
    /// keypair's pubkey
    #[clap(long = "recipient", value_parser = |p: &str| parse_address(p, "lamport_recipient_address"))]
    pub lamport_recipient_address: Option<Pubkey>,
}

#[derive(Clone, Debug, Args)]
#[clap(group(pool_source_group()))]
pub struct WithdrawCli {
    /// Amount of tokens to burn for withdrawal
    #[clap(value_parser = Amount::parse_decimal_or_all)]
    pub token_amount: Amount,

    /// The token account to withdraw from. Defaults to the associated token
    /// account for the pool mint
    #[clap(long = "token-account", value_parser = |p: &str| parse_address(p, "token_account_address"))]
    pub token_account_address: Option<Pubkey>,

    /// The pool to withdraw from
    #[clap(short, long = "pool", value_parser = |p: &str| parse_address(p, "pool_address"))]
    pub pool_address: Option<Pubkey>,

    /// The vote account corresponding to the pool to withdraw from
    #[clap(long = "vote-account", value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Option<Pubkey>,

    /// Signing authority on the token account. Defaults to the client keypair
    #[clap(long = "token-authority", id = "TOKEN_AUTHORITY_KEYPAIR", validator = |s| is_valid_signer(s))]
    pub token_authority: Option<SignerArg>,

    /// Authority to assign to the new stake account. Defaults to the pubkey of
    /// the client keypair
    #[clap(long = "stake-authority", value_parser = |p: &str| parse_address(p, "stake_authority_address"))]
    pub stake_authority_address: Option<Pubkey>,

    /// Deactivate stake account after withdrawal
    #[clap(long)]
    pub deactivate: bool,
}

#[derive(Clone, Debug, Args)]
#[clap(group(pool_source_group()))]
pub struct CreateMetadataCli {
    /// The pool to create default MPL token metadata for
    #[clap(short, long = "pool", value_parser = |p: &str| parse_address(p, "pool_address"))]
    pub pool_address: Option<Pubkey>,

    /// The vote account corresponding to the pool to create metadata for
    #[clap(long = "vote-account", value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Option<Pubkey>,
}

#[derive(Clone, Debug, Args)]
#[clap(group(pool_source_group()))]
pub struct UpdateMetadataCli {
    /// New name for the pool token
    #[clap(validator = is_valid_token_name)]
    pub token_name: String,

    /// New ticker symbol for the pool token
    #[clap(validator = is_valid_token_symbol)]
    pub token_symbol: String,

    /// Optional external URI for the pool token. Leaving this argument blank
    /// will clear any existing value
    #[clap(validator = is_valid_token_uri)]
    pub token_uri: Option<String>,

    /// The pool to change MPL token metadata for
    #[clap(short, long = "pool", value_parser = |p: &str| parse_address(p, "pool_address"))]
    pub pool_address: Option<Pubkey>,

    /// The vote account corresponding to the pool to create metadata for
    #[clap(long = "vote-account", value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Option<Pubkey>,

    /// Authorized withdrawer for the vote account, to prove validator
    /// ownership. Defaults to the client keypair
    #[clap(long, id = "AUTHORIZED_WITHDRAWER_KEYPAIR", validator = |s| is_valid_signer(s))]
    pub authorized_withdrawer: Option<SignerArg>,
}

#[derive(Clone, Debug, Args)]
#[clap(group(pool_source_group()))]
pub struct CreateStakeCli {
    /// Number of lamports to stake
    pub lamports: u64,

    /// The pool to create a stake account for
    #[clap(short, long = "pool", value_parser = |p: &str| parse_address(p, "pool_address"))]
    pub pool_address: Option<Pubkey>,

    /// The vote account corresponding to the pool to create stake for
    #[clap(long = "vote-account", value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Option<Pubkey>,

    /// Authority to assign to the new stake account. Defaults to the pubkey of
    /// the client keypair
    #[clap(long = "stake-authority", value_parser = |p: &str| parse_address(p, "stake_authority_address"))]
    pub stake_authority_address: Option<Pubkey>,
}

#[derive(Clone, Debug, Args)]
#[clap(group(pool_source_group().arg("all")))]
pub struct DisplayCli {
    /// The pool to display
    #[clap(value_parser = |p: &str| parse_address(p, "pool_address"))]
    pub pool_address: Option<Pubkey>,

    /// The vote account corresponding to the pool to display
    #[clap(long = "vote-account", value_parser = |p: &str| parse_address(p, "vote_account_address"))]
    pub vote_account_address: Option<Pubkey>,

    /// Display all pools
    #[clap(long)]
    pub all: bool,
}

fn pool_source_group() -> ArgGroup<'static> {
    ArgGroup::new("pool-source")
        .required(true)
        .args(&["pool-address", "vote-account-address"])
}

pub fn parse_address(path: &str, name: &str) -> Result<Pubkey, String> {
    if is_valid_pubkey(path).is_ok() {
        // this all is ugly but safe
        // wallet_manager doesn't need to be shared, it just saves cycles to cache it
        // and the only way argmatches default fails with an unchecked lookup is in the
        // prompt branch which seems unlikely to ever be used for pubkeys
        // the usb lookup in signer_from_path_with_config is safe
        // and the pubkey lookups are unreachable because pubkey_from_path short
        // circuits that case
        let mut wallet_manager = None;
        pubkey_from_path(&ArgMatches::default(), path, name, &mut wallet_manager)
            .map_err(|_| format!("Failed to load pubkey {} at {}", name, path))
    } else {
        Err(format!("Failed to parse pubkey {} at {}", name, path))
    }
}

pub fn parse_output_format(output_format: &str) -> OutputFormat {
    match output_format {
        "json" => OutputFormat::Json,
        "json-compact" => OutputFormat::JsonCompact,
        _ => unreachable!(),
    }
}

pub fn is_valid_token_name(s: &str) -> Result<(), String> {
    if s.len() > 32 {
        Err("Maximum token name length is 32 characters".to_string())
    } else {
        Ok(())
    }
}

pub fn is_valid_token_symbol(s: &str) -> Result<(), String> {
    if s.len() > 10 {
        Err("Maximum token symbol length is 10 characters".to_string())
    } else {
        Ok(())
    }
}

pub fn is_valid_token_uri(s: &str) -> Result<(), String> {
    if s.len() > 200 {
        Err("Maximum token URI length is 200 characters".to_string())
    } else {
        Ok(())
    }
}

pub fn pool_address_from_args(maybe_pool: Option<Pubkey>, maybe_vote: Option<Pubkey>) -> Pubkey {
    if let Some(pool_address) = maybe_pool {
        pool_address
    } else if let Some(vote_account_address) = maybe_vote {
        find_pool_address(&spl_single_pool::id(), &vote_account_address)
    } else {
        unreachable!()
    }
}

// all this is because solana clap v3 utils signer handlers dont work with
// derive syntax which means its impossible to parse keypairs or addresses in
// value_parser instead, we take the input into a string wrapper from the cli
// and then once the first pass is over, we do a second manual pass converting
// to signer wrappers
#[derive(Clone, Debug)]
pub enum SignerArg {
    Source(String),
    Signer(Arc<dyn Signer>),
}
impl FromStr for SignerArg {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::Source(s.to_string()))
    }
}
impl PartialEq for SignerArg {
    fn eq(&self, other: &SignerArg) -> bool {
        match (self, other) {
            (SignerArg::Source(ref a), SignerArg::Source(ref b)) => a == b,
            (SignerArg::Signer(ref a), SignerArg::Signer(ref b)) => a == b,
            (_, _) => false,
        }
    }
}

pub fn signer_from_arg(
    signer_arg: Option<SignerArg>,
    default_signer: &Arc<dyn Signer>,
) -> Result<Arc<dyn Signer>, Error> {
    match signer_arg {
        Some(SignerArg::Signer(signer)) => Ok(signer),
        Some(SignerArg::Source(_)) => Err("Signer arg string must be converted to signer".into()),
        None => Ok(default_signer.clone()),
    }
}

impl Command {
    pub fn with_signers(
        mut self,
        matches: &ArgMatches,
        wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
    ) -> Result<Self, Error> {
        match self {
            Command::Deposit(ref mut config) => {
                config.stake_withdraw_authority = with_signer(
                    matches,
                    wallet_manager,
                    config.stake_withdraw_authority.clone(),
                    "stake_authority",
                )?;
            }
            Command::Withdraw(ref mut config) => {
                config.token_authority = with_signer(
                    matches,
                    wallet_manager,
                    config.token_authority.clone(),
                    "token_authority",
                )?;
            }
            Command::Manage(ManageCli {
                manage: ManageCommand::UpdateTokenMetadata(ref mut config),
            }) => {
                config.authorized_withdrawer = with_signer(
                    matches,
                    wallet_manager,
                    config.authorized_withdrawer.clone(),
                    "authorized_withdrawer",
                )?;
            }
            _ => (),
        }

        Ok(self)
    }
}

pub fn with_signer(
    matches: &ArgMatches,
    wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
    arg: Option<SignerArg>,
    name: &str,
) -> Result<Option<SignerArg>, Error> {
    Ok(match arg {
        Some(SignerArg::Source(path)) => {
            let signer = if let Ok(signer) = signer_from_path(matches, &path, name, wallet_manager)
            {
                signer
            } else {
                return Err(format!("Cannot parse signer {} / {}", name, path).into());
            };
            Some(SignerArg::Signer(Arc::from(signer)))
        }
        a => a,
    })
}
