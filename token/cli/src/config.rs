use clap::ArgMatches;
use solana_clap_utils::{
    input_parsers::pubkey_of_signer,
    keypair::{signer_from_path_with_config, SignerFromPathConfig},
};
use solana_cli_output::OutputFormat;
use solana_client::{blockhash_query::BlockhashQuery, rpc_client::RpcClient};
use solana_remote_wallet::remote_wallet::RemoteWalletManager;
use solana_sdk::{pubkey::Pubkey, signature::Signer};
use spl_associated_token_account::*;
use std::{process::exit, sync::Arc};

#[cfg(test)]
use solana_sdk::signer::keypair::Keypair;

pub(crate) enum KeypairOrPath {
    /// Used for testing environments to avoid touching the filesystem
    #[cfg(test)]
    Keypair(Keypair),
    /// Used for real CLI usage
    Path(String),
}

pub(crate) struct Config<'a> {
    pub(crate) default_signer: Arc<dyn Signer>,
    pub(crate) default_address: Pubkey,
    pub(crate) rpc_client: Arc<RpcClient>,
    pub(crate) websocket_url: String,
    pub(crate) output_format: OutputFormat,
    pub(crate) fee_payer: Pubkey,
    pub(crate) default_keypair: KeypairOrPath,
    pub(crate) nonce_account: Option<Pubkey>,
    pub(crate) nonce_authority: Option<Pubkey>,
    pub(crate) blockhash_query: BlockhashQuery,
    pub(crate) sign_only: bool,
    pub(crate) dump_transaction_message: bool,
    pub(crate) multisigner_pubkeys: Vec<&'a Pubkey>,
    pub(crate) program_id: Pubkey,
}

impl<'a> Config<'a> {
    // Check if an explicit token account address was provided, otherwise
    // return the associated token address for the default address.
    pub(crate) fn associated_token_address_or_override(
        &self,
        arg_matches: &ArgMatches,
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
    }

    // Check if an explicit token account address was provided, otherwise
    // return the associated token address for the default address.
    pub(crate) fn associated_token_address_for_token_or_override(
        &self,
        arg_matches: &ArgMatches,
        override_name: &str,
        wallet_manager: &mut Option<Arc<RemoteWalletManager>>,
        token: Option<Pubkey>,
    ) -> Pubkey {
        if let Some(address) = pubkey_of_signer(arg_matches, override_name, wallet_manager).unwrap()
        {
            return address;
        }

        let token = token.unwrap();
        let owner = self.default_address;
        get_associated_token_address_with_program_id(&owner, &token, &self.program_id)
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

        return self.default_address.clone()
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
                        .map(|boxed| Arc::from(boxed))
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
}
