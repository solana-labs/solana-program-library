use crate::{signers_of, Error, MULTISIG_SIGNER_ARG};
use clap::ArgMatches;
use solana_clap_utils::{
    input_parsers::{pubkey_of, pubkey_of_signer, value_of},
    input_validators::normalize_to_url_if_moniker,
    keypair::{signer_from_path, signer_from_path_with_config, SignerFromPathConfig},
    nonce::{NONCE_ARG, NONCE_AUTHORITY_ARG},
    offline::{BLOCKHASH_ARG, DUMP_TRANSACTION_MESSAGE, SIGN_ONLY_ARG},
};
use solana_cli_output::OutputFormat;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_remote_wallet::remote_wallet::RemoteWalletManager;
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Signer};
use spl_associated_token_account::*;
use spl_token_2022::{
    extension::StateWithExtensionsOwned,
    state::{Account, Mint},
};
use spl_token_client::client::{
    ProgramClient, ProgramOfflineClient, ProgramRpcClient, ProgramRpcClientSendTransaction,
};
use std::{process::exit, sync::Arc};

pub(crate) struct MintInfo {
    pub program_id: Pubkey,
    pub address: Pubkey,
    pub decimals: u8,
}

pub(crate) struct Config<'a> {
    pub(crate) default_signer: Arc<dyn Signer>,
    pub(crate) rpc_client: Arc<RpcClient>,
    pub(crate) program_client: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>>,
    pub(crate) websocket_url: String,
    pub(crate) output_format: OutputFormat,
    pub(crate) fee_payer: Pubkey,
    pub(crate) nonce_account: Option<Pubkey>,
    pub(crate) nonce_authority: Option<Pubkey>,
    pub(crate) sign_only: bool,
    pub(crate) dump_transaction_message: bool,
    pub(crate) multisigner_pubkeys: Vec<&'a Pubkey>,
    pub(crate) program_id: Pubkey,
}

impl<'a> Config<'a> {
    pub(crate) fn new(
        matches: &ArgMatches,
        wallet_manager: &mut Option<Arc<RemoteWalletManager>>,
        bulk_signers: &mut Vec<Arc<dyn Signer>>,
        multisigner_ids: &'a mut Vec<Pubkey>,
    ) -> Self {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            solana_cli_config::Config::load(config_file).unwrap_or_else(|_| {
                eprintln!("error: Could not find config file `{}`", config_file);
                exit(1);
            })
        } else {
            solana_cli_config::Config::default()
        };
        let json_rpc_url = normalize_to_url_if_moniker(
            matches
                .value_of("json_rpc_url")
                .unwrap_or(&cli_config.json_rpc_url),
        );
        let websocket_url = solana_cli_config::Config::compute_websocket_url(&json_rpc_url);
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            json_rpc_url,
            CommitmentConfig::confirmed(),
        ));
        let sign_only = matches.is_present(SIGN_ONLY_ARG.name);
        let program_client: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>> = if sign_only {
            let blockhash = value_of(matches, BLOCKHASH_ARG.name);
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
    }

    pub(crate) fn new_with_clients_and_ws_url(
        matches: &ArgMatches,
        wallet_manager: &mut Option<Arc<RemoteWalletManager>>,
        bulk_signers: &mut Vec<Arc<dyn Signer>>,
        multisigner_ids: &'a mut Vec<Pubkey>,
        rpc_client: Arc<RpcClient>,
        program_client: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>>,
        websocket_url: String,
    ) -> Self {
        let cli_config = if let Some(config_file) = matches.value_of("config_file") {
            solana_cli_config::Config::load(config_file).unwrap_or_else(|_| {
                eprintln!("error: Could not find config file `{}`", config_file);
                exit(1);
            })
        } else {
            solana_cli_config::Config::default()
        };
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
        let multisigner_pubkeys = multisigner_ids.iter().collect::<Vec<_>>();

        let config = SignerFromPathConfig {
            allow_null_signer: !multisigner_pubkeys.is_empty(),
        };

        let default_keypair = cli_config.keypair_path.clone();

        let default_signer: Arc<dyn Signer> = {
            if let Some(owner_path) = matches.value_of("owner") {
                signer_from_path_with_config(matches, owner_path, "owner", wallet_manager, &config)
            } else {
                signer_from_path_with_config(
                    matches,
                    &default_keypair,
                    "default",
                    wallet_manager,
                    &config,
                )
            }
        }
        .map(Arc::from)
        .unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            exit(1);
        });

        let (signer, fee_payer) = matches
            .value_of("fee_payer")
            .map_or(Ok(default_signer.clone()), |path| {
                signer_from_path(matches, path, "fee_payer", wallet_manager).map(Arc::from)
            })
            .map(|s: Arc<dyn Signer>| {
                let p = s.pubkey();
                (s, p)
            })
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                exit(1);
            });
        bulk_signers.push(signer);

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

        let nonce_account = pubkey_of_signer(matches, NONCE_ARG.name, wallet_manager)
            .unwrap_or_else(|e| {
                eprintln!("error: {}", e);
                exit(1);
            });
        let nonce_authority = if nonce_account.is_some() {
            let (signer, nonce_authority) = signer_from_path(
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
            bulk_signers.push(signer);

            Some(nonce_authority)
        } else {
            None
        };

        let sign_only = matches.is_present(SIGN_ONLY_ARG.name);
        let dump_transaction_message = matches.is_present(DUMP_TRANSACTION_MESSAGE.name);
        let program_id = pubkey_of(matches, "program_id").unwrap();

        Self {
            default_signer,
            rpc_client,
            program_client,
            websocket_url,
            output_format,
            fee_payer,
            nonce_account,
            nonce_authority,
            sign_only,
            dump_transaction_message,
            multisigner_pubkeys,
            program_id,
        }
    }

    // Check if an explicit token account address was provided, otherwise
    // return the associated token address for the default address.
    pub(crate) async fn associated_token_address_or_override(
        &self,
        arg_matches: &ArgMatches<'_>,
        override_name: &str,
        wallet_manager: &mut Option<Arc<RemoteWalletManager>>,
    ) -> Pubkey {
        let token = pubkey_of_signer(arg_matches, "token", wallet_manager).unwrap();
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
        arg_matches: &ArgMatches<'_>,
        override_name: &str,
        wallet_manager: &mut Option<Arc<RemoteWalletManager>>,
        token: Option<Pubkey>,
    ) -> Pubkey {
        if let Some(address) = pubkey_of_signer(arg_matches, override_name, wallet_manager).unwrap()
        {
            return address;
        }

        let token = token.unwrap();
        let program_id = self.get_mint_info(&token, None).await.unwrap().program_id;
        self.associated_token_address_for_token_and_program(&token, &program_id)
    }

    pub(crate) fn associated_token_address_for_token_and_program(
        &self,
        token: &Pubkey,
        program_id: &Pubkey,
    ) -> Pubkey {
        let owner = self.default_signer.pubkey();
        get_associated_token_address_with_program_id(&owner, token, program_id)
    }

    // Checks if an explicit address was provided, otherwise return the default address.
    pub(crate) fn pubkey_or_default(
        &self,
        arg_matches: &ArgMatches,
        address_name: &str,
        wallet_manager: &mut Option<Arc<RemoteWalletManager>>,
    ) -> Pubkey {
        if address_name != "owner" {
            if let Some(address) =
                pubkey_of_signer(arg_matches, address_name, wallet_manager).unwrap()
            {
                return address;
            }
        }

        self.default_signer.pubkey()
    }

    // Checks if an explicit signer was provided, otherwise return the default signer.
    pub(crate) fn signer_or_default(
        &self,
        arg_matches: &ArgMatches,
        authority_name: &str,
        wallet_manager: &mut Option<Arc<RemoteWalletManager>>,
    ) -> (Arc<dyn Signer>, Pubkey) {
        // If there are `--multisig-signers` on the command line, allow `NullSigner`s to
        // be returned for multisig account addresses
        let config = SignerFromPathConfig {
            allow_null_signer: !self.multisigner_pubkeys.is_empty(),
        };
        let mut load_authority = move || -> Result<Arc<dyn Signer>, _> {
            if authority_name != "owner" {
                if let Some(keypair_path) = arg_matches.value_of(authority_name) {
                    return signer_from_path_with_config(
                        arg_matches,
                        keypair_path,
                        authority_name,
                        wallet_manager,
                        &config,
                    )
                    .map(Arc::from);
                }
            }

            Ok(self.default_signer.clone())
        };

        let authority = load_authority().unwrap_or_else(|e| {
            eprintln!("error: {}", e);
            exit(1);
        });

        let authority_address = authority.pubkey();
        (authority, authority_address)
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
            let account = self.rpc_client.get_account(mint).await?;
            self.check_owner(mint, &account.owner)?;
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

    pub(crate) fn check_owner(&self, account: &Pubkey, owner: &Pubkey) -> Result<(), Error> {
        if self.program_id != *owner {
            Err(format!(
                "Account {:?} is owned by {}, not configured program id {}",
                account, owner, self.program_id
            )
            .into())
        } else {
            Ok(())
        }
    }

    pub(crate) async fn check_account(
        &self,
        token_account: &Pubkey,
        mint_address: Option<Pubkey>,
    ) -> Result<Pubkey, Error> {
        if !self.sign_only {
            let account = self.rpc_client.get_account(token_account).await?;
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
            self.check_owner(token_account, &account.owner)?;
            Ok(source_mint)
        } else {
            Ok(mint_address.unwrap_or_default())
        }
    }
}
