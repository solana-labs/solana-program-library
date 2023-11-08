use {
    crate::cli::*,
    clap::ArgMatches,
    solana_clap_v3_utils::keypair::signer_from_path,
    solana_cli_output::OutputFormat,
    solana_client::nonblocking::rpc_client::RpcClient,
    solana_remote_wallet::remote_wallet::RemoteWalletManager,
    solana_sdk::{commitment_config::CommitmentConfig, signature::Signer},
    spl_token_client::client::{ProgramClient, ProgramRpcClient, ProgramRpcClientSendTransaction},
    std::{process::exit, rc::Rc, sync::Arc},
};

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub fn println_display(config: &Config, message: String) {
    match config.output_format {
        OutputFormat::Display | OutputFormat::DisplayVerbose => {
            println!("{}", message);
        }
        _ => {}
    }
}

pub fn eprintln_display(config: &Config, message: String) {
    match config.output_format {
        OutputFormat::Display | OutputFormat::DisplayVerbose => {
            eprintln!("{}", message);
        }
        _ => {}
    }
}

pub struct Config {
    pub rpc_client: Arc<RpcClient>,
    pub program_client: Arc<dyn ProgramClient<ProgramRpcClientSendTransaction>>,
    pub default_signer: Option<Arc<dyn Signer>>,
    pub fee_payer: Option<Arc<dyn Signer>>,
    pub output_format: OutputFormat,
    pub dry_run: bool,
}
impl Config {
    pub fn new(
        cli: Cli,
        matches: ArgMatches,
        wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
    ) -> Self {
        // get the generic cli config struct
        let cli_config = if let Some(config_file) = &cli.config_file {
            solana_cli_config::Config::load(config_file).unwrap_or_else(|_| {
                eprintln!("error: Could not load config file `{}`", config_file);
                exit(1);
            })
        } else if let Some(config_file) = &*solana_cli_config::CONFIG_FILE {
            solana_cli_config::Config::load(config_file).unwrap_or_default()
        } else {
            solana_cli_config::Config::default()
        };

        // create rpc client
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            cli.json_rpc_url.unwrap_or(cli_config.json_rpc_url),
            CommitmentConfig::confirmed(),
        ));

        // and program client
        let program_client = Arc::new(ProgramRpcClient::new(
            rpc_client.clone(),
            ProgramRpcClientSendTransaction,
        ));

        // resolve default signer
        let default_keypair = cli_config.keypair_path;
        let default_signer =
            signer_from_path(&matches, &default_keypair, "default", wallet_manager)
                .ok()
                .map(Arc::from);

        // resolve fee-payer
        let fee_payer_arg =
            with_signer(&matches, wallet_manager, cli.fee_payer, "fee_payer").unwrap();
        let fee_payer = default_signer
            .clone()
            .map(|default_signer| signer_from_arg(fee_payer_arg, &default_signer).unwrap());

        // determine output format
        let output_format = match (cli.output_format, cli.verbose) {
            (Some(json_format), _) => json_format,
            (None, true) => OutputFormat::DisplayVerbose,
            (None, false) => OutputFormat::Display,
        };

        Self {
            rpc_client,
            program_client,
            default_signer,
            fee_payer,
            output_format,
            dry_run: cli.dry_run,
        }
    }

    // Returns Ok(default signer), or Err if there is no default signer configured
    pub fn default_signer(&self) -> Result<Arc<dyn Signer>, Error> {
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
            Err("fee payer is required, please specify a valid fee payer using the --payer argument, or \
                 by identifying a valid configuration file using the --config argument, or by creating a \
                 valid config at the default location of ~/.config/solana/cli/config.yml using the solana \
                 config command".to_string().into())
        }
    }

    pub fn verbose(&self) -> bool {
        self.output_format == OutputFormat::DisplayVerbose
    }
}
