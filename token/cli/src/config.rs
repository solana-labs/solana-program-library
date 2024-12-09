use {
    crate::clap_app::{Error, COMPUTE_UNIT_LIMIT_ARG, COMPUTE_UNIT_PRICE_ARG, MULTISIG_SIGNER_ARG},
    clap::ArgMatches,
    solana_clap_v3_utils::{
        input_parsers::pubkey_of_signer,
        input_validators::normalize_to_url_if_moniker,
        keypair::SignerFromPathConfig,
        nonce::{NONCE_ARG, NONCE_AUTHORITY_ARG},
        offline::{BLOCKHASH_ARG, DUMP_TRANSACTION_MESSAGE, SIGNER_ARG, SIGN_ONLY_ARG},
    },
    solana_cli_output::OutputFormat,
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_remote_wallet::remote_wallet::RemoteWalletManager,
    solana_sdk::{
        account::Account as RawAccount, commitment_config::CommitmentConfig, hash::Hash,
        pubkey::Pubkey, signature::Signer, signer::null_signer::NullSigner,
    },
    spl_associated_token_account_client::address::get_associated_token_address_with_program_id,
    spl_token_2022::{
        extension::StateWithExtensionsOwned,
        state::{Account, Mint},
    },
    spl_token_client::{
        client::{
            ProgramClient, ProgramOfflineClient, ProgramRpcClient, ProgramRpcClientSendTransaction,
        },
        token::ComputeUnitLimit,
    },
    std::{process::exit, rc::Rc, str::FromStr, sync::Arc, time::Duration},
};

type SignersOf = Vec<(Arc<dyn Signer>, Pubkey)>;
fn signers_of(
    matches: &ArgMatches,
    name: &str,
    wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
) -> Result<Option<SignersOf>, Box<dyn std::error::Error>> {
    if let Some(values) = matches.values_of(name) {
        let mut results = Vec::new();
        for (i, value) in values.enumerate() {
            let name = format!("{}-{}", name, i.saturating_add(1));
            let signer = signer_from_path(matches, value, &name, wallet_manager)?;
            let signer_pubkey = signer.pubkey();
            results.push((Arc::from(signer), signer_pubkey));
        }
        Ok(Some(results))
    } else {
        Ok(None)
    }
}

pub(crate) struct MintInfo {
    pub program_id: Pubkey,
    pub address: Pubkey,
    pub decimals: u8,
}

const DEFAULT_RPC_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_CONFIRM_TX_TIMEOUT: Duration = Duration::from_secs(5);

pub struct Config<'a> {
    pub default_signer: Option<Arc<dyn Signer>>,
    pub rpc_client: Arc<RpcClient>,
    pub program_client: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>>,
    pub websocket_url: String,
    pub output_format: OutputFormat,
    pub fee_payer: Option<Arc<dyn Signer>>,
    pub nonce_account: Option<Pubkey>,
    pub nonce_authority: Option<Arc<dyn Signer>>,
    pub nonce_blockhash: Option<Hash>,
    pub sign_only: bool,
    pub dump_transaction_message: bool,
    pub multisigner_pubkeys: Vec<&'a Pubkey>,
    pub program_id: Pubkey,
    pub restrict_to_program_id: bool,
    pub compute_unit_price: Option<u64>,
    pub compute_unit_limit: ComputeUnitLimit,
}

impl<'a> Config<'a> {
    pub async fn new(
        matches: &ArgMatches,
        wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
        bulk_signers: &mut Vec<Arc<dyn Signer>>,
        multisigner_ids: &'a mut Vec<Pubkey>,
    ) -> Config<'a> {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            solana_cli_config::Config::load(config_file).unwrap_or_else(|_| {
                eprintln!("error: Could not find config file `{}`", config_file);
                exit(1);
            })
        } else if let Some(config_file) = &*solana_cli_config::CONFIG_FILE {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };
        let json_rpc_url = normalize_to_url_if_moniker(
            matches
                .value_of("json_rpc_url")
                .unwrap_or(&cli_config.json_rpc_url),
        );
        let websocket_url = solana_cli_config::Config::compute_websocket_url(&json_rpc_url);
        let rpc_client = Arc::new(RpcClient::new_with_timeouts_and_commitment(
            json_rpc_url,
            DEFAULT_RPC_TIMEOUT,
            CommitmentConfig::confirmed(),
            DEFAULT_CONFIRM_TX_TIMEOUT,
        ));
        let sign_only = matches.try_contains_id(SIGN_ONLY_ARG.name).unwrap_or(false);
        let program_client: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = if sign_only {
            let blockhash = matches
                .get_one::<Hash>(BLOCKHASH_ARG.name)
                .copied()
                .unwrap_or_default();
            Arc::new(ProgramOfflineClient::new(
                blockhash,
                ProgramRpcClientSendTransaction,
            ))
        } else {
            Arc::new(ProgramRpcClient::new(
                rpc_client.clone(),
                ProgramRpcClientSendTransaction,
            ))
        };
        Self::new_with_clients_and_ws_url(
            matches,
            wallet_manager,
            bulk_signers,
            multisigner_ids,
            rpc_client,
            program_client,
            websocket_url,
        )
        .await
    }

    fn extract_multisig_signers(
        matches: &ArgMatches,
        wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
        bulk_signers: &mut Vec<Arc<dyn Signer>>,
        multisigner_ids: &'a mut Vec<Pubkey>,
    ) -> Vec<&'a Pubkey> {
        let multisig_signers = signers_of(matches, MULTISIG_SIGNER_ARG.name, wallet_manager)
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                exit(1);
            });
        if let Some(mut multisig_signers) = multisig_signers {
            multisig_signers.sort_by(|(_, lp), (_, rp)| lp.cmp(rp));
            let (signers, pubkeys): (Vec<_>, Vec<_>) = multisig_signers.into_iter().unzip();
            bulk_signers.extend(signers);
            multisigner_ids.extend(pubkeys);
        }
        multisigner_ids.iter().collect::<Vec<_>>()
    }

    pub async fn new_with_clients_and_ws_url(
        matches: &ArgMatches,
        wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
        bulk_signers: &mut Vec<Arc<dyn Signer>>,
        multisigner_ids: &'a mut Vec<Pubkey>,
        rpc_client: Arc<RpcClient>,
        program_client: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>>,
        websocket_url: String,
    ) -> Config<'a> {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            solana_cli_config::Config::load(config_file).unwrap_or_else(|_| {
                eprintln!("error: Could not find config file `{}`", config_file);
                exit(1);
            })
        } else if let Some(config_file) = &*solana_cli_config::CONFIG_FILE {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };
        let multisigner_pubkeys =
            Self::extract_multisig_signers(matches, wallet_manager, bulk_signers, multisigner_ids);

        let config = SignerFromPathConfig {
            allow_null_signer: !multisigner_pubkeys.is_empty(),
        };

        let default_keypair = cli_config.keypair_path.clone();

        let default_signer: Option<Arc<dyn Signer>> = {
            if let Some(owner_path) = matches.try_get_one::<String>("owner").ok().flatten() {
                signer_from_path_with_config(matches, owner_path, "owner", wallet_manager, &config)
                    .ok()
            } else {
                signer_from_path_with_config(
                    matches,
                    &default_keypair,
                    "default",
                    wallet_manager,
                    &config,
                )
                .map_err(|e| {
                    if std::fs::metadata(&default_keypair).is_ok() {
                        eprintln!("error: {}", e);
                        exit(1);
                    } else {
                        e
                    }
                })
                .ok()
            }
        }
        .map(Arc::from);

        let fee_payer: Option<Arc<dyn Signer>> = matches
            .value_of("fee_payer")
            .map(|path| {
                Arc::from(
                    signer_from_path(matches, path, "fee_payer", wallet_manager).unwrap_or_else(
                        |e| {
                            eprintln!("error: {}", e);
                            exit(1);
                        },
                    ),
                )
            })
            .or_else(|| default_signer.clone());

        let verbose = matches.is_present("verbose");
        let output_format = matches
            .value_of("output_format")
            .map(|value| match value {
                "json" => OutputFormat::Json,
                "json-compact" => OutputFormat::JsonCompact,
                _ => unreachable!(),
            })
            .unwrap_or(if verbose {
                OutputFormat::DisplayVerbose
            } else {
                OutputFormat::Display
            });

        let nonce_account = match pubkey_of_signer(matches, NONCE_ARG.name, wallet_manager) {
            Ok(account) => account,
            Err(e) => {
                if e.is::<clap::parser::MatchesError>() {
                    None
                } else {
                    eprintln!("error: {}", e);
                    exit(1);
                }
            }
        };
        let nonce_authority = if nonce_account.is_some() {
            let (nonce_authority, _) = signer_from_path(
                matches,
                matches
                    .value_of(NONCE_AUTHORITY_ARG.name)
                    .unwrap_or(&cli_config.keypair_path),
                NONCE_AUTHORITY_ARG.name,
                wallet_manager,
            )
            .map(Arc::from)
            .map(|s: Arc<dyn Signer>| {
                let p = s.pubkey();
                (s, p)
            })
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                exit(1);
            });

            Some(nonce_authority)
        } else {
            None
        };

        let sign_only = matches.try_contains_id(SIGN_ONLY_ARG.name).unwrap_or(false);
        let dump_transaction_message = matches
            .try_contains_id(DUMP_TRANSACTION_MESSAGE.name)
            .unwrap_or(false);

        let pubkey_from_matches = |name| {
            matches
                .try_get_one::<String>(name)
                .ok()
                .flatten()
                .and_then(|pubkey| Pubkey::from_str(pubkey).ok())
        };

        let default_program_id = spl_token::id();
        let (program_id, restrict_to_program_id) = if matches.is_present("program_2022") {
            (spl_token_2022::id(), true)
        } else if let Some(program_id) = pubkey_from_matches("program_id") {
            (program_id, true)
        } else if !sign_only {
            if let Some(address) = pubkey_from_matches("token")
                .or_else(|| pubkey_from_matches("account"))
                .or_else(|| pubkey_from_matches("address"))
            {
                (
                    rpc_client
                        .get_account(&address)
                        .await
                        .map(|account| account.owner)
                        .unwrap_or(default_program_id),
                    false,
                )
            } else {
                (default_program_id, false)
            }
        } else {
            (default_program_id, false)
        };

        if matches.try_contains_id(BLOCKHASH_ARG.name).unwrap_or(false)
            && matches
                .try_contains_id(COMPUTE_UNIT_PRICE_ARG.name)
                .unwrap_or(false)
            && !matches
                .try_contains_id(COMPUTE_UNIT_LIMIT_ARG.name)
                .unwrap_or(false)
        {
            clap::Error::with_description(
                format!(
                    "Need to set `{}` if `{}` and `--{}` are set",
                    COMPUTE_UNIT_LIMIT_ARG.long, COMPUTE_UNIT_PRICE_ARG.long, BLOCKHASH_ARG.long,
                ),
                clap::ErrorKind::MissingRequiredArgument,
            )
            .exit();
        }

        let nonce_blockhash = matches
            .try_get_one::<Hash>(BLOCKHASH_ARG.name)
            .ok()
            .flatten()
            .copied();

        let compute_unit_price = matches.get_one::<u64>(COMPUTE_UNIT_PRICE_ARG.name).copied();

        let compute_unit_limit = matches
            .get_one::<u32>(COMPUTE_UNIT_LIMIT_ARG.name)
            .copied()
            .map(ComputeUnitLimit::Static)
            .unwrap_or_else(|| {
                if nonce_blockhash.is_some() {
                    ComputeUnitLimit::Default
                } else {
                    ComputeUnitLimit::Simulated
                }
            });

        Self {
            default_signer,
            rpc_client,
            program_client,
            websocket_url,
            output_format,
            fee_payer,
            nonce_account,
            nonce_authority,
            nonce_blockhash,
            sign_only,
            dump_transaction_message,
            multisigner_pubkeys,
            program_id,
            restrict_to_program_id,
            compute_unit_price,
            compute_unit_limit,
        }
    }

    // Returns Ok(default signer), or Err if there is no default signer configured
    pub(crate) fn default_signer(&self) -> Result<Arc<dyn Signer>, Error> {
        if let Some(default_signer) = &self.default_signer {
            Ok(default_signer.clone())
        } else {
            Err("default signer is required, please specify a valid default signer by identifying a \
                 valid configuration file using the --config argument, or by creating a valid config \
                 at the default location of ~/.config/solana/cli/config.yml using the solana config \
                 command".to_string().into())
        }
    }

    // Returns Ok(fee payer), or Err if there is no fee payer configured
    pub fn fee_payer(&self) -> Result<Arc<dyn Signer>, Error> {
        if let Some(fee_payer) = &self.fee_payer {
            Ok(fee_payer.clone())
        } else {
            Err("fee payer is required, please specify a valid fee payer using the --fee-payer argument, \
                 or by identifying a valid configuration file using the --config argument, or by creating \
                 a valid config at the default location of ~/.config/solana/cli/config.yml using the solana \
                 config command".to_string().into())
        }
    }

    // Check if an explicit token account address was provided, otherwise
    // return the associated token address for the default address.
    pub(crate) async fn associated_token_address_or_override(
        &self,
        arg_matches: &ArgMatches,
        override_name: &str,
        wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
    ) -> Result<Pubkey, Error> {
        let token = pubkey_of_signer(arg_matches, "token", wallet_manager)
            .map_err(|e| -> Error { e.to_string().into() })?;
        self.associated_token_address_for_token_or_override(
            arg_matches,
            override_name,
            wallet_manager,
            token,
        )
        .await
    }

    // Check if an explicit token account address was provided, otherwise
    // return the associated token address for the default address.
    pub(crate) async fn associated_token_address_for_token_or_override(
        &self,
        arg_matches: &ArgMatches,
        override_name: &str,
        wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
        token: Option<Pubkey>,
    ) -> Result<Pubkey, Error> {
        if let Some(address) = pubkey_of_signer(arg_matches, override_name, wallet_manager)
            .map_err(|e| -> Error { e.to_string().into() })?
        {
            return Ok(address);
        }

        let token = token.unwrap();
        let program_id = self.get_mint_info(&token, None).await?.program_id;
        let owner = self.pubkey_or_default(arg_matches, "owner", wallet_manager)?;
        self.associated_token_address_for_token_and_program(&token, &owner, &program_id)
    }

    pub(crate) fn associated_token_address_for_token_and_program(
        &self,
        token: &Pubkey,
        owner: &Pubkey,
        program_id: &Pubkey,
    ) -> Result<Pubkey, Error> {
        Ok(get_associated_token_address_with_program_id(
            owner, token, program_id,
        ))
    }

    // Checks if an explicit address was provided, otherwise return the default
    // address if there is one
    pub(crate) fn pubkey_or_default(
        &self,
        arg_matches: &ArgMatches,
        address_name: &str,
        wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
    ) -> Result<Pubkey, Error> {
        if let Some(address) = pubkey_of_signer(arg_matches, address_name, wallet_manager)
            .map_err(|e| -> Error { e.to_string().into() })?
        {
            return Ok(address);
        }

        Ok(self.default_signer()?.pubkey())
    }

    // Checks if an explicit signer was provided, otherwise return the default
    // signer.
    pub(crate) fn signer_or_default(
        &self,
        arg_matches: &ArgMatches,
        authority_name: &str,
        wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
    ) -> (Arc<dyn Signer>, Pubkey) {
        // If there are `--multisig-signers` on the command line, allow `NullSigner`s to
        // be returned for multisig account addresses
        let config = SignerFromPathConfig {
            allow_null_signer: !self.multisigner_pubkeys.is_empty(),
        };
        let mut load_authority = move || -> Result<Arc<dyn Signer>, Error> {
            if authority_name != "owner" {
                if let Some(keypair_path) = arg_matches.value_of(authority_name) {
                    return signer_from_path_with_config(
                        arg_matches,
                        keypair_path,
                        authority_name,
                        wallet_manager,
                        &config,
                    )
                    .map(Arc::from)
                    .map_err(|e| e.to_string().into());
                }
            }

            self.default_signer()
        };

        let authority = load_authority().unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            exit(1);
        });

        let authority_address = authority.pubkey();
        (authority, authority_address)
    }

    pub(crate) async fn get_account_checked(
        &self,
        account_pubkey: &Pubkey,
    ) -> Result<RawAccount, Error> {
        if let Ok(Some(account)) = self.program_client.get_account(*account_pubkey).await {
            if self.program_id == account.owner {
                Ok(account)
            } else {
                Err(format!(
                    "Account {} is owned by {}, not configured program id {}",
                    account_pubkey, account.owner, self.program_id
                )
                .into())
            }
        } else {
            Err(format!("Account {} not found", account_pubkey).into())
        }
    }

    pub(crate) async fn get_mint_info(
        &self,
        mint: &Pubkey,
        mint_decimals: Option<u8>,
    ) -> Result<MintInfo, Error> {
        if self.sign_only {
            Ok(MintInfo {
                program_id: self.program_id,
                address: *mint,
                decimals: mint_decimals.unwrap_or_default(),
            })
        } else {
            let account = self.get_account_checked(mint).await?;
            let mint_account = StateWithExtensionsOwned::<Mint>::unpack(account.data)
                .map_err(|_| format!("Could not find mint account {}", mint))?;
            if let Some(decimals) = mint_decimals {
                if decimals != mint_account.base.decimals {
                    return Err(format!(
                        "Mint {:?} has decimals {}, not configured decimals {}",
                        mint, mint_account.base.decimals, decimals
                    )
                    .into());
                }
            }
            Ok(MintInfo {
                program_id: account.owner,
                address: *mint,
                decimals: mint_account.base.decimals,
            })
        }
    }

    pub(crate) async fn check_account(
        &self,
        token_account: &Pubkey,
        mint_address: Option<Pubkey>,
    ) -> Result<Pubkey, Error> {
        if !self.sign_only {
            let account = self.get_account_checked(token_account).await?;
            let source_account = StateWithExtensionsOwned::<Account>::unpack(account.data)
                .map_err(|_| format!("Could not find token account {}", token_account))?;
            let source_mint = source_account.base.mint;
            if let Some(mint) = mint_address {
                if source_mint != mint {
                    return Err(format!(
                        "Source {:?} does not contain {:?} tokens",
                        token_account, mint
                    )
                    .into());
                }
            }
            Ok(source_mint)
        } else {
            Ok(mint_address.unwrap_or_default())
        }
    }
}

// In clap v2, `value_of` returns `None` if the argument id is not previously
// specified in `Arg`. In contrast, in clap v3, `value_of` panics in this case.
// Therefore, compared to the same function in solana-clap-utils,
// `signer_from_path` in solana-clap-v3-utils errors early when `path` is a
// valid pubkey, but `SIGNER_ARG.name` is not specified in the args.
// This function behaves exactly as `signer_from_path` from solana-clap-utils by
// catching this special case.
fn signer_from_path(
    matches: &ArgMatches,
    path: &str,
    keypair_name: &str,
    wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
) -> Result<Box<dyn Signer>, Box<dyn std::error::Error>> {
    let config = SignerFromPathConfig::default();
    signer_from_path_with_config(matches, path, keypair_name, wallet_manager, &config)
}

fn signer_from_path_with_config(
    matches: &ArgMatches,
    path: &str,
    keypair_name: &str,
    wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
    config: &SignerFromPathConfig,
) -> Result<Box<dyn Signer>, Box<dyn std::error::Error>> {
    if let Ok(pubkey) = Pubkey::from_str(path) {
        if matches.try_contains_id(SIGNER_ARG.name).is_err()
            && (config.allow_null_signer || matches.try_contains_id(SIGN_ONLY_ARG.name)?)
        {
            return Ok(Box::new(NullSigner::new(&pubkey)));
        }
    }

    solana_clap_v3_utils::keypair::signer_from_path_with_config(
        matches,
        path,
        keypair_name,
        wallet_manager,
        config,
    )
}
