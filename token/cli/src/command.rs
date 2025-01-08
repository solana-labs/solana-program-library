#![allow(clippy::arithmetic_side_effects)]
use {
    crate::{
        bench::*,
        clap_app::*,
        config::{Config, MintInfo},
        encryption_keypair::*,
        output::*,
        print_error_and_exit,
        sort::{sort_and_parse_token_accounts, AccountFilter},
    },
    clap::{value_t, value_t_or_exit, ArgMatches},
    futures::try_join,
    serde::Serialize,
    solana_account_decoder::{
        parse_account_data::SplTokenAdditionalData,
        parse_token::{get_token_account_mint, parse_token_v2, TokenAccountType, UiAccountState},
        UiAccountData,
    },
    solana_clap_v3_utils::{
        input_parsers::{pubkey_of_signer, signer::SignerSource, Amount},
        keypair::signer_from_path,
    },
    solana_cli_output::{
        return_signers_data, CliSignOnlyData, CliSignature, OutputFormat, QuietDisplay,
        ReturnSignersConfig, VerboseDisplay,
    },
    solana_client::rpc_request::TokenAccountsFilter,
    solana_remote_wallet::remote_wallet::RemoteWalletManager,
    solana_sdk::{
        instruction::AccountMeta,
        native_token::*,
        program_option::COption,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        system_program,
    },
    spl_associated_token_account_client::address::get_associated_token_address_with_program_id,
    spl_token_2022::{
        extension::{
            confidential_transfer::{
                account_info::{
                    ApplyPendingBalanceAccountInfo, TransferAccountInfo, WithdrawAccountInfo,
                },
                ConfidentialTransferAccount, ConfidentialTransferMint,
            },
            confidential_transfer_fee::ConfidentialTransferFeeConfig,
            cpi_guard::CpiGuard,
            default_account_state::DefaultAccountState,
            group_member_pointer::GroupMemberPointer,
            group_pointer::GroupPointer,
            interest_bearing_mint::InterestBearingConfig,
            memo_transfer::MemoTransfer,
            metadata_pointer::MetadataPointer,
            mint_close_authority::MintCloseAuthority,
            permanent_delegate::PermanentDelegate,
            transfer_fee::{TransferFeeAmount, TransferFeeConfig},
            transfer_hook::TransferHook,
            BaseStateWithExtensions, ExtensionType, StateWithExtensionsOwned,
        },
        solana_zk_sdk::encryption::{
            auth_encryption::AeKey,
            elgamal::{self, ElGamalKeypair},
            pod::elgamal::PodElGamalPubkey,
        },
        state::{Account, AccountState, Mint},
    },
    spl_token_client::{
        client::{ProgramRpcClientSendTransaction, RpcClientResponse},
        token::{
            ComputeUnitLimit, ExtensionInitializationParams, ProofAccount,
            ProofAccountWithCiphertext, Token,
        },
    },
    spl_token_confidential_transfer_proof_generation::{
        transfer::TransferProofData, withdraw::WithdrawProofData,
    },
    spl_token_group_interface::state::TokenGroup,
    spl_token_metadata_interface::state::{Field, TokenMetadata},
    std::{collections::HashMap, fmt::Display, process::exit, rc::Rc, str::FromStr, sync::Arc},
};

fn amount_to_raw_amount(amount: Amount, decimals: u8, all_amount: Option<u64>, name: &str) -> u64 {
    match amount {
        Amount::Raw(ui_amount) => ui_amount,
        Amount::Decimal(ui_amount) => spl_token::ui_amount_to_amount(ui_amount, decimals),
        Amount::All => {
            if let Some(raw_amount) = all_amount {
                raw_amount
            } else {
                eprintln!("ALL keyword is not allowed for {}", name);
                exit(1)
            }
        }
    }
}

type BulkSigners = Vec<Arc<dyn Signer>>;
pub type CommandResult = Result<String, Error>;

fn push_signer_with_dedup(signer: Arc<dyn Signer>, bulk_signers: &mut BulkSigners) {
    if !bulk_signers.contains(&signer) {
        bulk_signers.push(signer);
    }
}

fn new_throwaway_signer() -> (Arc<dyn Signer>, Pubkey) {
    let keypair = Keypair::new();
    let pubkey = keypair.pubkey();
    (Arc::new(keypair) as Arc<dyn Signer>, pubkey)
}

fn get_signer(
    matches: &ArgMatches,
    keypair_name: &str,
    wallet_manager: &mut Option<Rc<RemoteWalletManager>>,
) -> Option<(Arc<dyn Signer>, Pubkey)> {
    matches.value_of(keypair_name).map(|path| {
        let signer = signer_from_path(matches, path, keypair_name, wallet_manager)
            .unwrap_or_else(print_error_and_exit);
        let signer_pubkey = signer.pubkey();
        (Arc::from(signer), signer_pubkey)
    })
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

fn base_token_client(
    config: &Config<'_>,
    token_pubkey: &Pubkey,
    decimals: Option<u8>,
) -> Result<Token<ProgramRpcClientSendTransaction>, Error> {
    Ok(Token::new(
        config.program_client.clone(),
        &config.program_id,
        token_pubkey,
        decimals,
        config.fee_payer()?.clone(),
    ))
}

fn config_token_client(
    token: Token<ProgramRpcClientSendTransaction>,
    config: &Config<'_>,
) -> Result<Token<ProgramRpcClientSendTransaction>, Error> {
    let token = token.with_compute_unit_limit(config.compute_unit_limit.clone());

    let token = if let Some(compute_unit_price) = config.compute_unit_price {
        token.with_compute_unit_price(compute_unit_price)
    } else {
        token
    };

    if let (Some(nonce_account), Some(nonce_authority), Some(nonce_blockhash)) = (
        config.nonce_account,
        &config.nonce_authority,
        config.nonce_blockhash,
    ) {
        Ok(token.with_nonce(
            &nonce_account,
            Arc::clone(nonce_authority),
            &nonce_blockhash,
        ))
    } else {
        Ok(token)
    }
}

fn token_client_from_config(
    config: &Config<'_>,
    token_pubkey: &Pubkey,
    decimals: Option<u8>,
) -> Result<Token<ProgramRpcClientSendTransaction>, Error> {
    let token = base_token_client(config, token_pubkey, decimals)?;
    config_token_client(token, config)
}

fn native_token_client_from_config(
    config: &Config<'_>,
) -> Result<Token<ProgramRpcClientSendTransaction>, Error> {
    let token = Token::new_native(
        config.program_client.clone(),
        &config.program_id,
        config.fee_payer()?.clone(),
    );

    let token = token.with_compute_unit_limit(config.compute_unit_limit.clone());

    let token = if let Some(compute_unit_price) = config.compute_unit_price {
        token.with_compute_unit_price(compute_unit_price)
    } else {
        token
    };

    if let (Some(nonce_account), Some(nonce_authority), Some(nonce_blockhash)) = (
        config.nonce_account,
        &config.nonce_authority,
        config.nonce_blockhash,
    ) {
        Ok(token.with_nonce(
            &nonce_account,
            Arc::clone(nonce_authority),
            &nonce_blockhash,
        ))
    } else {
        Ok(token)
    }
}

#[derive(strum_macros::Display, Debug)]
#[strum(serialize_all = "kebab-case")]
enum Pointer {
    Metadata,
    Group,
    GroupMember,
}

#[allow(clippy::too_many_arguments)]
async fn command_create_token(
    config: &Config<'_>,
    decimals: u8,
    token_pubkey: Pubkey,
    authority: Pubkey,
    enable_freeze: bool,
    enable_close: bool,
    enable_non_transferable: bool,
    enable_permanent_delegate: bool,
    memo: Option<String>,
    metadata_address: Option<Pubkey>,
    group_address: Option<Pubkey>,
    member_address: Option<Pubkey>,
    rate_bps: Option<i16>,
    default_account_state: Option<AccountState>,
    transfer_fee: Option<(u16, u64)>,
    confidential_transfer_auto_approve: Option<bool>,
    transfer_hook_program_id: Option<Pubkey>,
    enable_metadata: bool,
    enable_group: bool,
    enable_member: bool,
    bulk_signers: Vec<Arc<dyn Signer>>,
) -> CommandResult {
    println_display(
        config,
        format!(
            "Creating token {} under program {}",
            token_pubkey, config.program_id
        ),
    );

    let token = token_client_from_config(config, &token_pubkey, Some(decimals))?;

    let freeze_authority = if enable_freeze { Some(authority) } else { None };

    let mut extensions = vec![];

    if enable_close {
        extensions.push(ExtensionInitializationParams::MintCloseAuthority {
            close_authority: Some(authority),
        });
    }

    if enable_permanent_delegate {
        extensions.push(ExtensionInitializationParams::PermanentDelegate {
            delegate: authority,
        });
    }

    if let Some(rate_bps) = rate_bps {
        extensions.push(ExtensionInitializationParams::InterestBearingConfig {
            rate_authority: Some(authority),
            rate: rate_bps,
        })
    }

    if enable_non_transferable {
        extensions.push(ExtensionInitializationParams::NonTransferable);
    }

    if let Some(state) = default_account_state {
        assert!(
            enable_freeze,
            "Token requires a freeze authority to default to frozen accounts"
        );
        extensions.push(ExtensionInitializationParams::DefaultAccountState { state })
    }

    if let Some((transfer_fee_basis_points, maximum_fee)) = transfer_fee {
        extensions.push(ExtensionInitializationParams::TransferFeeConfig {
            transfer_fee_config_authority: Some(authority),
            withdraw_withheld_authority: Some(authority),
            transfer_fee_basis_points,
            maximum_fee,
        });
    }

    if let Some(auto_approve) = confidential_transfer_auto_approve {
        extensions.push(ExtensionInitializationParams::ConfidentialTransferMint {
            authority: Some(authority),
            auto_approve_new_accounts: auto_approve,
            auditor_elgamal_pubkey: None,
        });
        if transfer_fee.is_some() {
            // Deriving ElGamal key from default signer. Custom ElGamal keys
            // will be supported in the future once upgrading to clap-v3.
            //
            // NOTE: Seed bytes are hardcoded to be empty bytes for now. They
            // will be updated once custom ElGamal keys are supported.
            let elgamal_keypair =
                ElGamalKeypair::new_from_signer(config.default_signer()?.as_ref(), b"").unwrap();
            extensions.push(
                ExtensionInitializationParams::ConfidentialTransferFeeConfig {
                    authority: Some(authority),
                    withdraw_withheld_authority_elgamal_pubkey: (*elgamal_keypair.pubkey()).into(),
                },
            );
        }
    }

    if let Some(program_id) = transfer_hook_program_id {
        extensions.push(ExtensionInitializationParams::TransferHook {
            authority: Some(authority),
            program_id: Some(program_id),
        });
    }

    if let Some(text) = memo {
        token.with_memo(text, vec![config.default_signer()?.pubkey()]);
    }

    // CLI checks that only one is set
    if metadata_address.is_some() || enable_metadata {
        let metadata_address = if enable_metadata {
            Some(token_pubkey)
        } else {
            metadata_address
        };
        extensions.push(ExtensionInitializationParams::MetadataPointer {
            authority: Some(authority),
            metadata_address,
        });
    }

    if group_address.is_some() || enable_group {
        let group_address = if enable_group {
            Some(token_pubkey)
        } else {
            group_address
        };
        extensions.push(ExtensionInitializationParams::GroupPointer {
            authority: Some(authority),
            group_address,
        });
    }

    if member_address.is_some() || enable_member {
        let member_address = if enable_member {
            Some(token_pubkey)
        } else {
            member_address
        };
        extensions.push(ExtensionInitializationParams::GroupMemberPointer {
            authority: Some(authority),
            member_address,
        });
    }

    let res = token
        .create_mint(
            &authority,
            freeze_authority.as_ref(),
            extensions,
            &bulk_signers,
        )
        .await?;

    let tx_return = finish_tx(config, &res, false).await?;

    if enable_metadata {
        println_display(
            config,
            format!(
                "To initialize metadata inside the mint, please run \
                `spl-token initialize-metadata {token_pubkey} <YOUR_TOKEN_NAME> <YOUR_TOKEN_SYMBOL> <YOUR_TOKEN_URI>`, \
                and sign with the mint authority.",
            ),
        );
    }

    if enable_group {
        println_display(
            config,
            format!(
                "To initialize group configurations inside the mint, please run `spl-token initialize-group {token_pubkey} <MAX_SIZE>`, and sign with the mint authority.",
            ),
        );
    }

    if enable_member {
        println_display(
            config,
            format!(
                "To initialize group member configurations inside the mint, please run `spl-token initialize-member {token_pubkey}`, and sign with the mint authority and the group's update authority.",
            ),
        );
    }

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
    let mut token = token_client_from_config(config, &token_pubkey, None)?;
    // Because set_interest_rate depends on the time, it can cost more between
    // simulation and execution. To help that, just set a static compute limit
    // if none has been set
    if !matches!(config.compute_unit_limit, ComputeUnitLimit::Static(_)) {
        token = token.with_compute_unit_limit(ComputeUnitLimit::Static(2_500));
    }

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

async fn command_set_transfer_hook_program(
    config: &Config<'_>,
    token_pubkey: Pubkey,
    authority: Pubkey,
    new_program_id: Option<Pubkey>,
    bulk_signers: Vec<Arc<dyn Signer>>,
) -> CommandResult {
    let token = token_client_from_config(config, &token_pubkey, None)?;

    if !config.sign_only {
        let mint_account = config.get_account_checked(&token_pubkey).await?;

        let mint_state = StateWithExtensionsOwned::<Mint>::unpack(mint_account.data)
            .map_err(|_| format!("Could not deserialize token mint {}", token_pubkey))?;

        if let Ok(extension) = mint_state.get_extension::<TransferHook>() {
            let authority_pubkey = Option::<Pubkey>::from(extension.authority);

            if authority_pubkey != Some(authority) {
                return Err(format!(
                    "Mint {} has transfer hook authority {}, but {} was provided",
                    token_pubkey,
                    authority_pubkey
                        .map(|pubkey| pubkey.to_string())
                        .unwrap_or_else(|| "disabled".to_string()),
                    authority
                )
                .into());
            }
        } else {
            return Err(
                format!("Mint {} does not have permissioned-transfers", token_pubkey).into(),
            );
        }
    }

    println_display(
        config,
        format!(
            "Setting Transfer Hook Program id for {} to {}",
            token_pubkey,
            new_program_id
                .map(|pubkey| pubkey.to_string())
                .unwrap_or_else(|| "disabled".to_string())
        ),
    );

    let res = token
        .update_transfer_hook_program_id(&authority, new_program_id, &bulk_signers)
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
async fn command_initialize_metadata(
    config: &Config<'_>,
    token_pubkey: Pubkey,
    update_authority: Pubkey,
    mint_authority: Pubkey,
    name: String,
    symbol: String,
    uri: String,
    bulk_signers: Vec<Arc<dyn Signer>>,
) -> CommandResult {
    let token = token_client_from_config(config, &token_pubkey, None)?;

    let res = token
        .token_metadata_initialize_with_rent_transfer(
            &config.fee_payer()?.pubkey(),
            &update_authority,
            &mint_authority,
            name,
            symbol,
            uri,
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

async fn command_update_metadata(
    config: &Config<'_>,
    token_pubkey: Pubkey,
    authority: Pubkey,
    field: Field,
    value: Option<String>,
    transfer_lamports: Option<u64>,
    bulk_signers: Vec<Arc<dyn Signer>>,
) -> CommandResult {
    let token = token_client_from_config(config, &token_pubkey, None)?;

    let res = if let Some(value) = value {
        token
            .token_metadata_update_field_with_rent_transfer(
                &config.fee_payer()?.pubkey(),
                &authority,
                field,
                value,
                transfer_lamports,
                &bulk_signers,
            )
            .await?
    } else if let Field::Key(key) = field {
        token
            .token_metadata_remove_key(
                &authority,
                key,
                true, // idempotent
                &bulk_signers,
            )
            .await?
    } else {
        return Err(format!(
            "Attempting to remove field {field:?}, which cannot be removed. \
            Please re-run the command with a value of \"\" rather than the `--remove` flag."
        )
        .into());
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

#[allow(clippy::too_many_arguments)]
async fn command_initialize_group(
    config: &Config<'_>,
    token_pubkey: Pubkey,
    mint_authority: Pubkey,
    update_authority: Pubkey,
    max_size: u64,
    bulk_signers: Vec<Arc<dyn Signer>>,
) -> CommandResult {
    let token = token_client_from_config(config, &token_pubkey, None)?;

    let res = token
        .token_group_initialize_with_rent_transfer(
            &config.fee_payer()?.pubkey(),
            &mint_authority,
            &update_authority,
            max_size,
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

#[allow(clippy::too_many_arguments)]
async fn command_update_group_max_size(
    config: &Config<'_>,
    token_pubkey: Pubkey,
    update_authority: Pubkey,
    new_max_size: u64,
    bulk_signers: Vec<Arc<dyn Signer>>,
) -> CommandResult {
    let token = token_client_from_config(config, &token_pubkey, None)?;

    let res = token
        .token_group_update_max_size(&update_authority, new_max_size, &bulk_signers)
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

async fn command_initialize_member(
    config: &Config<'_>,
    member_token_pubkey: Pubkey,
    mint_authority: Pubkey,
    group_token_pubkey: Pubkey,
    group_update_authority: Pubkey,
    bulk_signers: Vec<Arc<dyn Signer>>,
) -> CommandResult {
    let token = token_client_from_config(config, &member_token_pubkey, None)?;

    let res = token
        .token_group_initialize_member_with_rent_transfer(
            &config.fee_payer()?.pubkey(),
            &mint_authority,
            &group_token_pubkey,
            &group_update_authority,
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

async fn command_set_transfer_fee(
    config: &Config<'_>,
    token_pubkey: Pubkey,
    transfer_fee_authority: Pubkey,
    transfer_fee_basis_points: u16,
    maximum_fee: Amount,
    mint_decimals: Option<u8>,
    bulk_signers: Vec<Arc<dyn Signer>>,
) -> CommandResult {
    let decimals = if !config.sign_only {
        let mint_account = config.get_account_checked(&token_pubkey).await?;

        let mint_state = StateWithExtensionsOwned::<Mint>::unpack(mint_account.data)
            .map_err(|_| format!("Could not deserialize token mint {}", token_pubkey))?;

        if mint_decimals.is_some() && mint_decimals != Some(mint_state.base.decimals) {
            return Err(format!(
                "Decimals {} was provided, but actual value is {}",
                mint_decimals.unwrap(),
                mint_state.base.decimals
            )
            .into());
        }

        if let Ok(transfer_fee_config) = mint_state.get_extension::<TransferFeeConfig>() {
            let mint_fee_authority_pubkey =
                Option::<Pubkey>::from(transfer_fee_config.transfer_fee_config_authority);

            if mint_fee_authority_pubkey != Some(transfer_fee_authority) {
                return Err(format!(
                    "Mint {} has transfer fee authority {}, but {} was provided",
                    token_pubkey,
                    mint_fee_authority_pubkey
                        .map(|pubkey| pubkey.to_string())
                        .unwrap_or_else(|| "disabled".to_string()),
                    transfer_fee_authority
                )
                .into());
            }
        } else {
            return Err(format!("Mint {} does not have a transfer fee", token_pubkey).into());
        }
        mint_state.base.decimals
    } else {
        mint_decimals.unwrap()
    };

    let token = token_client_from_config(config, &token_pubkey, Some(decimals))?;
    let maximum_fee = amount_to_raw_amount(maximum_fee, decimals, None, "MAXIMUM_FEE");

    println_display(
        config,
        format!(
            "Setting transfer fee for {} to {} bps, {} maximum",
            token_pubkey,
            transfer_fee_basis_points,
            spl_token::amount_to_ui_amount(maximum_fee, decimals)
        ),
    );

    let res = token
        .set_transfer_fee(
            &transfer_fee_authority,
            transfer_fee_basis_points,
            maximum_fee,
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

async fn command_create_account(
    config: &Config<'_>,
    token_pubkey: Pubkey,
    owner: Pubkey,
    maybe_account: Option<Pubkey>,
    immutable_owner: bool,
    bulk_signers: Vec<Arc<dyn Signer>>,
) -> CommandResult {
    let token = token_client_from_config(config, &token_pubkey, None)?;
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

    // default is safe here because create_multisig doesn't use it
    let token = token_client_from_config(config, &Pubkey::default(), None)?;

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
    authority_type: CliAuthorityType,
    authority: Pubkey,
    new_authority: Option<Pubkey>,
    force_authorize: bool,
    bulk_signers: BulkSigners,
) -> CommandResult {
    let auth_str: &'static str = (&authority_type).into();

    let (mint_pubkey, previous_authority) = if !config.sign_only {
        let target_account = config.get_account_checked(&account).await?;

        let (mint_pubkey, previous_authority) = if let Ok(mint) =
            StateWithExtensionsOwned::<Mint>::unpack(target_account.data.clone())
        {
            let previous_authority = match authority_type {
                CliAuthorityType::Owner | CliAuthorityType::Close => Err(format!(
                    "Authority type `{}` not supported for SPL Token mints",
                    auth_str
                )),
                CliAuthorityType::Mint => Ok(Option::<Pubkey>::from(mint.base.mint_authority)),
                CliAuthorityType::Freeze => Ok(Option::<Pubkey>::from(mint.base.freeze_authority)),
                CliAuthorityType::CloseMint => {
                    if let Ok(mint_close_authority) = mint.get_extension::<MintCloseAuthority>() {
                        Ok(Option::<Pubkey>::from(mint_close_authority.close_authority))
                    } else {
                        Err(format!(
                            "Mint `{}` does not support close authority",
                            account
                        ))
                    }
                }
                CliAuthorityType::TransferFeeConfig => {
                    if let Ok(transfer_fee_config) = mint.get_extension::<TransferFeeConfig>() {
                        Ok(Option::<Pubkey>::from(
                            transfer_fee_config.transfer_fee_config_authority,
                        ))
                    } else {
                        Err(format!("Mint `{}` does not support transfer fees", account))
                    }
                }
                CliAuthorityType::WithheldWithdraw => {
                    if let Ok(transfer_fee_config) = mint.get_extension::<TransferFeeConfig>() {
                        Ok(Option::<Pubkey>::from(
                            transfer_fee_config.withdraw_withheld_authority,
                        ))
                    } else {
                        Err(format!("Mint `{}` does not support transfer fees", account))
                    }
                }
                CliAuthorityType::InterestRate => {
                    if let Ok(interest_rate_config) = mint.get_extension::<InterestBearingConfig>()
                    {
                        Ok(Option::<Pubkey>::from(interest_rate_config.rate_authority))
                    } else {
                        Err(format!("Mint `{}` is not interest-bearing", account))
                    }
                }
                CliAuthorityType::PermanentDelegate => {
                    if let Ok(permanent_delegate) = mint.get_extension::<PermanentDelegate>() {
                        Ok(Option::<Pubkey>::from(permanent_delegate.delegate))
                    } else {
                        Err(format!(
                            "Mint `{}` does not support permanent delegate",
                            account
                        ))
                    }
                }
                CliAuthorityType::ConfidentialTransferMint => {
                    if let Ok(confidential_transfer_mint) =
                        mint.get_extension::<ConfidentialTransferMint>()
                    {
                        Ok(Option::<Pubkey>::from(confidential_transfer_mint.authority))
                    } else {
                        Err(format!(
                            "Mint `{}` does not support confidential transfers",
                            account
                        ))
                    }
                }
                CliAuthorityType::TransferHookProgramId => {
                    if let Ok(extension) = mint.get_extension::<TransferHook>() {
                        Ok(Option::<Pubkey>::from(extension.authority))
                    } else {
                        Err(format!(
                            "Mint `{}` does not support a transfer hook program",
                            account
                        ))
                    }
                }
                CliAuthorityType::ConfidentialTransferFee => {
                    if let Ok(confidential_transfer_fee_config) =
                        mint.get_extension::<ConfidentialTransferFeeConfig>()
                    {
                        Ok(Option::<Pubkey>::from(
                            confidential_transfer_fee_config.authority,
                        ))
                    } else {
                        Err(format!(
                            "Mint `{}` does not support confidential transfer fees",
                            account
                        ))
                    }
                }
                CliAuthorityType::MetadataPointer => {
                    if let Ok(extension) = mint.get_extension::<MetadataPointer>() {
                        Ok(Option::<Pubkey>::from(extension.authority))
                    } else {
                        Err(format!(
                            "Mint `{}` does not support a metadata pointer",
                            account
                        ))
                    }
                }
                CliAuthorityType::Metadata => {
                    if let Ok(extension) = mint.get_variable_len_extension::<TokenMetadata>() {
                        Ok(Option::<Pubkey>::from(extension.update_authority))
                    } else {
                        Err(format!("Mint `{account}` does not support metadata"))
                    }
                }
                CliAuthorityType::GroupPointer => {
                    if let Ok(extension) = mint.get_extension::<GroupPointer>() {
                        Ok(Option::<Pubkey>::from(extension.authority))
                    } else {
                        Err(format!(
                            "Mint `{}` does not support a group pointer",
                            account
                        ))
                    }
                }
                CliAuthorityType::GroupMemberPointer => {
                    if let Ok(extension) = mint.get_extension::<GroupMemberPointer>() {
                        Ok(Option::<Pubkey>::from(extension.authority))
                    } else {
                        Err(format!(
                            "Mint `{}` does not support a group member pointer",
                            account
                        ))
                    }
                }
                CliAuthorityType::Group => {
                    if let Ok(extension) = mint.get_extension::<TokenGroup>() {
                        Ok(Option::<Pubkey>::from(extension.update_authority))
                    } else {
                        Err(format!("Mint `{}` does not support token groups", account))
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
                CliAuthorityType::Mint
                | CliAuthorityType::Freeze
                | CliAuthorityType::CloseMint
                | CliAuthorityType::TransferFeeConfig
                | CliAuthorityType::WithheldWithdraw
                | CliAuthorityType::InterestRate
                | CliAuthorityType::PermanentDelegate
                | CliAuthorityType::ConfidentialTransferMint
                | CliAuthorityType::TransferHookProgramId
                | CliAuthorityType::ConfidentialTransferFee
                | CliAuthorityType::MetadataPointer
                | CliAuthorityType::Metadata
                | CliAuthorityType::GroupPointer
                | CliAuthorityType::Group
                | CliAuthorityType::GroupMemberPointer => Err(format!(
                    "Authority type `{auth_str}` not supported for SPL Token accounts",
                )),
                CliAuthorityType::Owner => {
                    check_associated_token_account()?;
                    Ok(Some(token_account.base.owner))
                }
                CliAuthorityType::Close => {
                    check_associated_token_account()?;
                    Ok(Some(
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
        // default is safe here because authorize doesn't use it
        (Pubkey::default(), None)
    };

    let token = token_client_from_config(config, &mint_pubkey, None)?;

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

    let res = match authority_type {
        CliAuthorityType::Metadata => {
            token
                .token_metadata_update_authority(&authority, new_authority, &bulk_signers)
                .await?
        }
        CliAuthorityType::Group => {
            token
                .token_group_update_authority(&authority, new_authority, &bulk_signers)
                .await?
        }
        _ => {
            token
                .set_authority(
                    &account,
                    &authority,
                    new_authority.as_ref(),
                    authority_type.try_into()?,
                    &bulk_signers,
                )
                .await?
        }
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

#[allow(clippy::too_many_arguments)]
async fn command_transfer(
    config: &Config<'_>,
    token_pubkey: Pubkey,
    ui_amount: Amount,
    recipient: Pubkey,
    sender: Option<Pubkey>,
    sender_owner: Pubkey,
    allow_unfunded_recipient: bool,
    fund_recipient: bool,
    mint_decimals: Option<u8>,
    no_recipient_is_ata_owner: bool,
    use_unchecked_instruction: bool,
    ui_fee: Option<Amount>,
    memo: Option<String>,
    bulk_signers: BulkSigners,
    no_wait: bool,
    allow_non_system_account_recipient: bool,
    transfer_hook_accounts: Option<Vec<AccountMeta>>,
    confidential_transfer_args: Option<&ConfidentialTransferArgs>,
) -> CommandResult {
    let mint_info = config.get_mint_info(&token_pubkey, mint_decimals).await?;

    // if the user got the decimals wrong, they may well have calculated the
    // transfer amount wrong we only check in online mode, because in offline,
    // mint_info.decimals is always 9
    if !config.sign_only && mint_decimals.is_some() && mint_decimals != Some(mint_info.decimals) {
        return Err(format!(
            "Decimals {} was provided, but actual value is {}",
            mint_decimals.unwrap(),
            mint_info.decimals
        )
        .into());
    }

    // decimals determines whether transfer_checked is used or not
    // in online mode, mint_decimals may be None but mint_info.decimals is always
    // correct in offline mode, mint_info.decimals may be wrong, but
    // mint_decimals is always provided and in online mode, when mint_decimals
    // is provided, it is verified correct hence the fallthrough logic here
    let decimals = if use_unchecked_instruction {
        None
    } else if mint_decimals.is_some() {
        mint_decimals
    } else {
        Some(mint_info.decimals)
    };

    let token = if let Some(transfer_hook_accounts) = transfer_hook_accounts {
        token_client_from_config(config, &token_pubkey, decimals)?
            .with_transfer_hook_accounts(transfer_hook_accounts)
    } else if config.sign_only {
        // we need to pass in empty transfer hook accounts on sign-only,
        // otherwise the token client will try to fetch the mint account and fail
        token_client_from_config(config, &token_pubkey, decimals)?
            .with_transfer_hook_accounts(vec![])
    } else {
        token_client_from_config(config, &token_pubkey, decimals)?
    };

    // pubkey of the actual account we are sending from
    let sender = if let Some(sender) = sender {
        sender
    } else {
        token.get_associated_token_address(&sender_owner)
    };

    // the sender balance
    let sender_balance = if config.sign_only {
        None
    } else {
        Some(token.get_account_info(&sender).await?.base.amount)
    };

    // the amount the user wants to transfer, as a u64
    let transfer_balance = match ui_amount {
        Amount::Raw(ui_amount) => ui_amount,
        Amount::Decimal(ui_amount) => spl_token::ui_amount_to_amount(ui_amount, mint_info.decimals),
        Amount::All => {
            if config.sign_only {
                return Err("Use of ALL keyword to burn tokens requires online signing"
                    .to_string()
                    .into());
            }
            sender_balance.unwrap()
        }
    };

    println_display(
        config,
        format!(
            "{}Transfer {} tokens\n  Sender: {}\n  Recipient: {}",
            if confidential_transfer_args.is_some() {
                "Confidential "
            } else {
                ""
            },
            spl_token::amount_to_ui_amount(transfer_balance, mint_info.decimals),
            sender,
            recipient
        ),
    );

    if let Some(sender_balance) = sender_balance {
        if transfer_balance > sender_balance && confidential_transfer_args.is_none() {
            return Err(format!(
                "Error: Sender has insufficient funds, current balance is {}",
                spl_token_2022::amount_to_ui_amount_string_trimmed(
                    sender_balance,
                    mint_info.decimals
                )
            )
            .into());
        }
    }

    let maybe_fee =
        ui_fee.map(|v| amount_to_raw_amount(v, mint_info.decimals, None, "EXPECTED_FEE"));

    // determine whether recipient is a token account or an expected owner of one
    let recipient_is_token_account = if !config.sign_only {
        // in online mode we can fetch it and see
        let maybe_recipient_account_data = config.program_client.get_account(recipient).await?;

        // if the account exists, and:
        // * its a token for this program, we are happy
        // * its a system account, we are happy
        // * its a non-account for this program, we error helpfully
        // * its a token account for a different program, we error helpfully
        // * otherwise its probably a program account owner of an ata, in which case we
        //   gate transfer with a flag
        if let Some(recipient_account_data) = maybe_recipient_account_data {
            let recipient_account_owner = recipient_account_data.owner;
            let maybe_account_state =
                StateWithExtensionsOwned::<Account>::unpack(recipient_account_data.data);

            if recipient_account_owner == config.program_id && maybe_account_state.is_ok() {
                if let Ok(memo_transfer) = maybe_account_state?.get_extension::<MemoTransfer>() {
                    if memo_transfer.require_incoming_transfer_memos.into() && memo.is_none() {
                        return Err(
                            "Error: Recipient expects a transfer memo, but none was provided. \
                                    Provide a memo using `--with-memo`."
                                .into(),
                        );
                    }
                }

                true
            } else if recipient_account_owner == system_program::id() {
                false
            } else if recipient_account_owner == config.program_id {
                return Err(
                    "Error: Recipient is owned by this token program, but is not a token account."
                        .into(),
                );
            } else if VALID_TOKEN_PROGRAM_IDS.contains(&recipient_account_owner) {
                return Err(format!(
                    "Error: Recipient is owned by {}, but the token mint is owned by {}.",
                    recipient_account_owner, config.program_id
                )
                .into());
            } else if allow_non_system_account_recipient {
                false
            } else {
                return Err("Error: The recipient address is not owned by the System Program. \
                                     Add `--allow-non-system-account-recipient` to complete the transfer.".into());
            }
        }
        // if it doesn't exist, it definitely isn't a token account!
        // we gate transfer with a different flag
        else if maybe_recipient_account_data.is_none() && allow_unfunded_recipient {
            false
        } else {
            return Err("Error: The recipient address is not funded. \
                        Add `--allow-unfunded-recipient` to complete the transfer."
                .into());
        }
    } else {
        // in offline mode we gotta trust them
        no_recipient_is_ata_owner
    };

    // now if its a token account, life is ez
    let (recipient_token_account, fundable_owner) = if recipient_is_token_account {
        (recipient, None)
    }
    // but if not, we need to determine if we can or should create an ata for recipient
    else {
        // first, get the ata address
        let recipient_token_account = token.get_associated_token_address(&recipient);

        println_display(
            config,
            format!(
                "  Recipient associated token account: {}",
                recipient_token_account
            ),
        );

        // if we can fetch it to determine if it exists, do so
        let needs_funding = if !config.sign_only {
            if let Some(recipient_token_account_data) = config
                .program_client
                .get_account(recipient_token_account)
                .await?
            {
                let recipient_token_account_owner = recipient_token_account_data.owner;

                if let Ok(account_state) =
                    StateWithExtensionsOwned::<Account>::unpack(recipient_token_account_data.data)
                {
                    if let Ok(memo_transfer) = account_state.get_extension::<MemoTransfer>() {
                        if memo_transfer.require_incoming_transfer_memos.into() && memo.is_none() {
                            return Err(
                                "Error: Recipient expects a transfer memo, but none was provided. \
                                        Provide a memo using `--with-memo`."
                                    .into(),
                            );
                        }
                    }
                }

                if recipient_token_account_owner == system_program::id() {
                    true
                } else if recipient_token_account_owner == config.program_id {
                    false
                } else {
                    return Err(
                        format!("Error: Unsupported recipient address: {}", recipient).into(),
                    );
                }
            } else {
                true
            }
        }
        // otherwise trust the cli flag
        else {
            fund_recipient
        };

        // and now we determine if we will actually fund it, based on its need and our
        // willingness
        let fundable_owner = if needs_funding {
            if confidential_transfer_args.is_some() {
                return Err(
                    "Error: Recipient's associated token account does not exist. \
                        Accounts cannot be funded for confidential transfers."
                        .into(),
                );
            } else if fund_recipient {
                println_display(
                    config,
                    format!("  Funding recipient: {}", recipient_token_account,),
                );

                Some(recipient)
            } else {
                return Err(
                    "Error: Recipient's associated token account does not exist. \
                                    Add `--fund-recipient` to fund their account"
                        .into(),
                );
            }
        } else {
            None
        };

        (recipient_token_account, fundable_owner)
    };

    // set up memo if provided...
    if let Some(text) = memo {
        token.with_memo(text, vec![config.default_signer()?.pubkey()]);
    }

    // fetch confidential transfer info for recipient and auditor
    let (recipient_elgamal_pubkey, auditor_elgamal_pubkey) = if let Some(args) =
        confidential_transfer_args
    {
        if !config.sign_only {
            // we can use the mint data from the start of the function, but will require
            // non-trivial amount of refactoring the code due to ownership; for now, we
            // fetch the mint a second time. This can potentially be optimized
            // in the future.
            let confidential_transfer_mint = config.get_account_checked(&token_pubkey).await?;
            let mint_state =
                StateWithExtensionsOwned::<Mint>::unpack(confidential_transfer_mint.data)
                    .map_err(|_| format!("Could not deserialize token mint {}", token_pubkey))?;

            let auditor_elgamal_pubkey = if let Ok(confidential_transfer_mint) =
                mint_state.get_extension::<ConfidentialTransferMint>()
            {
                let expected_auditor_elgamal_pubkey = Option::<PodElGamalPubkey>::from(
                    confidential_transfer_mint.auditor_elgamal_pubkey,
                );

                // if auditor ElGamal pubkey is provided, check consistency with the one in the
                // mint if auditor ElGamal pubkey is not provided, then use the
                // expected one from the   mint, which could also be `None` if
                // auditing is disabled
                if args.auditor_elgamal_pubkey.is_some()
                    && expected_auditor_elgamal_pubkey != args.auditor_elgamal_pubkey
                {
                    return Err(format!(
                        "Mint {} has confidential transfer auditor {}, but {} was provided",
                        token_pubkey,
                        expected_auditor_elgamal_pubkey
                            .map(|pubkey| pubkey.to_string())
                            .unwrap_or_else(|| "disabled".to_string()),
                        args.auditor_elgamal_pubkey.unwrap(),
                    )
                    .into());
                }

                expected_auditor_elgamal_pubkey
            } else {
                return Err(format!(
                    "Mint {} does not support confidential transfers",
                    token_pubkey
                )
                .into());
            };

            let recipient_account = config.get_account_checked(&recipient_token_account).await?;
            let recipient_elgamal_pubkey =
                StateWithExtensionsOwned::<Account>::unpack(recipient_account.data)?
                    .get_extension::<ConfidentialTransferAccount>()?
                    .elgamal_pubkey;

            (Some(recipient_elgamal_pubkey), auditor_elgamal_pubkey)
        } else {
            let recipient_elgamal_pubkey = args
                .recipient_elgamal_pubkey
                .expect("Recipient ElGamal pubkey must be provided");
            let auditor_elgamal_pubkey = args
                .auditor_elgamal_pubkey
                .expect("Auditor ElGamal pubkey must be provided");

            (Some(recipient_elgamal_pubkey), Some(auditor_elgamal_pubkey))
        }
    } else {
        (None, None)
    };

    // ...and, finally, the transfer
    let res = match (fundable_owner, maybe_fee, confidential_transfer_args) {
        (Some(recipient_owner), None, None) => {
            token
                .create_recipient_associated_account_and_transfer(
                    &sender,
                    &recipient_token_account,
                    &recipient_owner,
                    &sender_owner,
                    transfer_balance,
                    maybe_fee,
                    &bulk_signers,
                )
                .await?
        }
        (Some(_), _, _) => {
            panic!("Recipient account cannot be created for transfer with fees or confidential transfers");
        }
        (None, Some(fee), None) => {
            token
                .transfer_with_fee(
                    &sender,
                    &recipient_token_account,
                    &sender_owner,
                    transfer_balance,
                    fee,
                    &bulk_signers,
                )
                .await?
        }
        (None, None, Some(args)) => {
            // deserialize `pod` ElGamal pubkeys
            let recipient_elgamal_pubkey: elgamal::ElGamalPubkey = recipient_elgamal_pubkey
                .unwrap()
                .try_into()
                .expect("Invalid recipient ElGamal pubkey");
            let auditor_elgamal_pubkey = auditor_elgamal_pubkey.map(|pubkey| {
                let auditor_elgamal_pubkey: elgamal::ElGamalPubkey =
                    pubkey.try_into().expect("Invalid auditor ElGamal pubkey");
                auditor_elgamal_pubkey
            });

            let context_state_authority = config.fee_payer()?;
            let context_state_authority_pubkey = context_state_authority.pubkey();
            let equality_proof_context_state_account = Keypair::new();
            let equality_proof_pubkey = equality_proof_context_state_account.pubkey();
            let ciphertext_validity_proof_context_state_account = Keypair::new();
            let ciphertext_validity_proof_pubkey =
                ciphertext_validity_proof_context_state_account.pubkey();
            let range_proof_context_state_account = Keypair::new();
            let range_proof_pubkey = range_proof_context_state_account.pubkey();

            let state = token.get_account_info(&sender).await.unwrap();
            let extension = state
                .get_extension::<ConfidentialTransferAccount>()
                .unwrap();
            let transfer_account_info = TransferAccountInfo::new(extension);

            let TransferProofData {
                equality_proof_data,
                ciphertext_validity_proof_data_with_ciphertext,
                range_proof_data,
            } = transfer_account_info
                .generate_split_transfer_proof_data(
                    transfer_balance,
                    &args.sender_elgamal_keypair,
                    &args.sender_aes_key,
                    &recipient_elgamal_pubkey,
                    auditor_elgamal_pubkey.as_ref(),
                )
                .unwrap();

            let transfer_amount_auditor_ciphertext_lo =
                ciphertext_validity_proof_data_with_ciphertext.ciphertext_lo;
            let transfer_amount_auditor_ciphertext_hi =
                ciphertext_validity_proof_data_with_ciphertext.ciphertext_hi;

            // setup proofs
            let create_range_proof_context_signer = &[&range_proof_context_state_account];
            let create_equality_proof_context_signer = &[&equality_proof_context_state_account];
            let create_ciphertext_validity_proof_context_signer =
                &[&ciphertext_validity_proof_context_state_account];

            let _ = try_join!(
                token.confidential_transfer_create_context_state_account(
                    &range_proof_pubkey,
                    &context_state_authority_pubkey,
                    &range_proof_data,
                    true,
                    create_range_proof_context_signer
                ),
                token.confidential_transfer_create_context_state_account(
                    &equality_proof_pubkey,
                    &context_state_authority_pubkey,
                    &equality_proof_data,
                    false,
                    create_equality_proof_context_signer
                ),
                token.confidential_transfer_create_context_state_account(
                    &ciphertext_validity_proof_pubkey,
                    &context_state_authority_pubkey,
                    &ciphertext_validity_proof_data_with_ciphertext.proof_data,
                    false,
                    create_ciphertext_validity_proof_context_signer
                )
            )?;

            // do the transfer
            let equality_proof_context_proof_account =
                ProofAccount::ContextAccount(equality_proof_pubkey);
            let ciphertext_validity_proof_context_proof_account =
                ProofAccount::ContextAccount(ciphertext_validity_proof_pubkey);
            let range_proof_context_proof_account =
                ProofAccount::ContextAccount(range_proof_pubkey);

            let ciphertext_validity_proof_account_with_ciphertext = ProofAccountWithCiphertext {
                proof_account: ciphertext_validity_proof_context_proof_account,
                ciphertext_lo: transfer_amount_auditor_ciphertext_lo,
                ciphertext_hi: transfer_amount_auditor_ciphertext_hi,
            };

            let transfer_result = token
                .confidential_transfer_transfer(
                    &sender,
                    &recipient_token_account,
                    &sender_owner,
                    Some(&equality_proof_context_proof_account),
                    Some(&ciphertext_validity_proof_account_with_ciphertext),
                    Some(&range_proof_context_proof_account),
                    transfer_balance,
                    Some(transfer_account_info),
                    &args.sender_elgamal_keypair,
                    &args.sender_aes_key,
                    &recipient_elgamal_pubkey,
                    auditor_elgamal_pubkey.as_ref(),
                    &bulk_signers,
                )
                .await?;

            // close context state accounts
            let close_context_state_signer = &[&context_state_authority];
            let _ = try_join!(
                token.confidential_transfer_close_context_state_account(
                    &equality_proof_pubkey,
                    &sender,
                    &context_state_authority_pubkey,
                    close_context_state_signer
                ),
                token.confidential_transfer_close_context_state_account(
                    &ciphertext_validity_proof_pubkey,
                    &sender,
                    &context_state_authority_pubkey,
                    close_context_state_signer
                ),
                token.confidential_transfer_close_context_state_account(
                    &range_proof_pubkey,
                    &sender,
                    &context_state_authority_pubkey,
                    close_context_state_signer
                ),
            )?;

            transfer_result
        }
        (None, Some(_), Some(_)) => {
            panic!("Confidential transfer with fee is not yet supported.");
        }
        (None, None, None) => {
            token
                .transfer(
                    &sender,
                    &recipient_token_account,
                    &sender_owner,
                    transfer_balance,
                    &bulk_signers,
                )
                .await?
        }
    };

    let tx_return = finish_tx(config, &res, no_wait).await?;
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
    ui_amount: Amount,
    mint_address: Option<Pubkey>,
    mint_decimals: Option<u8>,
    use_unchecked_instruction: bool,
    memo: Option<String>,
    bulk_signers: BulkSigners,
) -> CommandResult {
    let mint_address = config.check_account(&account, mint_address).await?;
    let mint_info = config.get_mint_info(&mint_address, mint_decimals).await?;
    let decimals = if use_unchecked_instruction {
        None
    } else {
        Some(mint_info.decimals)
    };

    let token = token_client_from_config(config, &mint_info.address, decimals)?;

    let amount = match ui_amount {
        Amount::Raw(ui_amount) => ui_amount,
        Amount::Decimal(ui_amount) => spl_token::ui_amount_to_amount(ui_amount, mint_info.decimals),
        Amount::All => {
            if config.sign_only {
                return Err("Use of ALL keyword to burn tokens requires online signing"
                    .to_string()
                    .into());
            }
            token.get_account_info(&account).await?.base.amount
        }
    };

    println_display(
        config,
        format!(
            "Burn {} tokens\n  Source: {}",
            spl_token::amount_to_ui_amount(amount, mint_info.decimals),
            account
        ),
    );

    if let Some(text) = memo {
        token.with_memo(text, vec![config.default_signer()?.pubkey()]);
    }

    let res = token.burn(&account, &owner, amount, &bulk_signers).await?;

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
    ui_amount: Amount,
    recipient: Pubkey,
    mint_info: MintInfo,
    mint_authority: Pubkey,
    use_unchecked_instruction: bool,
    memo: Option<String>,
    bulk_signers: BulkSigners,
) -> CommandResult {
    let amount = amount_to_raw_amount(ui_amount, mint_info.decimals, None, "TOKEN_AMOUNT");

    println_display(
        config,
        format!(
            "Minting {} tokens\n  Token: {}\n  Recipient: {}",
            spl_token::amount_to_ui_amount(amount, mint_info.decimals),
            token,
            recipient
        ),
    );

    let decimals = if use_unchecked_instruction {
        None
    } else {
        Some(mint_info.decimals)
    };

    let token = token_client_from_config(config, &mint_info.address, decimals)?;
    if let Some(text) = memo {
        token.with_memo(text, vec![config.default_signer()?.pubkey()]);
    }

    let res = token
        .mint_to(&recipient, &mint_authority, amount, &bulk_signers)
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

    // we dont use the decimals from mint_info because its not need and in sign-only
    // its wrong
    let token = token_client_from_config(config, &mint_info.address, None)?;
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

    // we dont use the decimals from mint_info because its not need and in sign-only
    // its wrong
    let token = token_client_from_config(config, &mint_info.address, None)?;
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
    amount: Amount,
    wallet_address: Pubkey,
    wrapped_sol_account: Option<Pubkey>,
    immutable_owner: bool,
    bulk_signers: BulkSigners,
) -> CommandResult {
    let lamports = match amount {
        Amount::Raw(amount) => amount,
        Amount::Decimal(amount) => sol_to_lamports(amount),
        Amount::All => {
            return Err("ALL keyword not supported for SOL amount".into());
        }
    };
    let token = native_token_client_from_config(config)?;

    let account =
        wrapped_sol_account.unwrap_or_else(|| token.get_associated_token_address(&wallet_address));

    println_display(
        config,
        format!(
            "Wrapping {} SOL into {}",
            lamports_to_sol(lamports),
            account
        ),
    );

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
        // this case is hit for a token22 ata, which is always immutable. but it does
        // the right thing anyway
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
    let token = native_token_client_from_config(config)?;

    let account =
        maybe_account.unwrap_or_else(|| token.get_associated_token_address(&wallet_address));

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
    ui_amount: Amount,
    delegate: Pubkey,
    mint_address: Option<Pubkey>,
    mint_decimals: Option<u8>,
    use_unchecked_instruction: bool,
    bulk_signers: BulkSigners,
) -> CommandResult {
    let mint_address = config.check_account(&account, mint_address).await?;
    let mint_info = config.get_mint_info(&mint_address, mint_decimals).await?;
    let amount = amount_to_raw_amount(ui_amount, mint_info.decimals, None, "TOKEN_AMOUNT");
    let decimals = if use_unchecked_instruction {
        None
    } else {
        Some(mint_info.decimals)
    };

    println_display(
        config,
        format!(
            "Approve {} tokens\n  Account: {}\n  Delegate: {}",
            spl_token::amount_to_ui_amount(amount, mint_info.decimals),
            account,
            delegate
        ),
    );

    let token = token_client_from_config(config, &mint_info.address, decimals)?;
    let res = token
        .approve(&account, &delegate, &owner, amount, &bulk_signers)
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
        // default is safe here because revoke doesn't use it
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

    let token = token_client_from_config(config, &mint_pubkey, None)?;
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
    let mut results = vec![];
    let token = if !config.sign_only {
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

        let token = token_client_from_config(config, &source_state.base.mint, None)?;
        if let Ok(extension) = source_state.get_extension::<TransferFeeAmount>() {
            if u64::from(extension.withheld_amount) != 0 {
                let res = token.harvest_withheld_tokens_to_mint(&[&account]).await?;
                let tx_return = finish_tx(config, &res, false).await?;
                results.push(match tx_return {
                    TransactionReturnData::CliSignature(signature) => {
                        config.output_format.formatted_string(&signature)
                    }
                    TransactionReturnData::CliSignOnlyData(sign_only_data) => {
                        config.output_format.formatted_string(&sign_only_data)
                    }
                });
            }
        }

        token
    } else {
        // default is safe here because close doesn't use it
        token_client_from_config(config, &Pubkey::default(), None)?
    };

    let res = token
        .close_account(&account, &recipient, &close_authority, &bulk_signers)
        .await?;

    let tx_return = finish_tx(config, &res, false).await?;
    results.push(match tx_return {
        TransactionReturnData::CliSignature(signature) => {
            config.output_format.formatted_string(&signature)
        }
        TransactionReturnData::CliSignOnlyData(sign_only_data) => {
            config.output_format.formatted_string(&sign_only_data)
        }
    });
    Ok(results.join(""))
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

    let token = token_client_from_config(config, &token_pubkey, None)?;
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
    maybe_token: Option<Pubkey>,
    owner: Pubkey,
    account_filter: AccountFilter,
    print_addresses_only: bool,
) -> CommandResult {
    let filters = if let Some(token_pubkey) = maybe_token {
        let _ = config.get_mint_info(&token_pubkey, None).await?;
        vec![TokenAccountsFilter::Mint(token_pubkey)]
    } else if config.restrict_to_program_id {
        vec![TokenAccountsFilter::ProgramId(config.program_id)]
    } else {
        vec![
            TokenAccountsFilter::ProgramId(spl_token::id()),
            TokenAccountsFilter::ProgramId(spl_token_2022::id()),
        ]
    };

    let mut accounts = vec![];
    for filter in filters {
        accounts.push(
            config
                .rpc_client
                .get_token_accounts_by_owner(&owner, filter)
                .await?,
        );
    }
    let accounts = accounts.into_iter().flatten().collect();

    let cli_token_accounts =
        sort_and_parse_token_accounts(&owner, accounts, maybe_token.is_some(), account_filter)?;

    if print_addresses_only {
        Ok(cli_token_accounts
            .accounts
            .into_iter()
            .flatten()
            .map(|a| a.address)
            .collect::<Vec<_>>()
            .join("\n"))
    } else {
        Ok(config.output_format.formatted_string(&cli_token_accounts))
    }
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
        config.get_mint_info(&token, None).await?;
        let associated_token_address =
            get_associated_token_address_with_program_id(&owner, &token, &config.program_id);
        cli_address.associated_token_address = Some(associated_token_address.to_string());
    }
    Ok(config.output_format.formatted_string(&cli_address))
}

async fn command_display(config: &Config<'_>, address: Pubkey) -> CommandResult {
    let account_data = config.get_account_checked(&address).await?;

    let (additional_data, has_permanent_delegate) =
        if let Some(mint_address) = get_token_account_mint(&account_data.data) {
            let mint_account = config.get_account_checked(&mint_address).await?;
            let mint_state = StateWithExtensionsOwned::<Mint>::unpack(mint_account.data)
                .map_err(|_| format!("Could not deserialize token mint {}", mint_address))?;

            let has_permanent_delegate =
                if let Ok(permanent_delegate) = mint_state.get_extension::<PermanentDelegate>() {
                    Option::<Pubkey>::from(permanent_delegate.delegate).is_some()
                } else {
                    false
                };
            let additional_data = SplTokenAdditionalData::with_decimals(mint_state.base.decimals);

            (Some(additional_data), has_permanent_delegate)
        } else {
            (None, false)
        };

    let token_data = parse_token_v2(&account_data.data, additional_data.as_ref());

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
                is_associated: associated_address == address,
                account,
                has_permanent_delegate,
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

    let mut accounts_by_token = HashMap::new();

    for keyed_account in accounts {
        if let UiAccountData::Json(parsed_account) = keyed_account.account.data {
            if let Ok(TokenAccountType::Account(ui_token_account)) =
                serde_json::from_value(parsed_account.parsed)
            {
                let frozen = ui_token_account.state == UiAccountState::Frozen;
                let decimals = ui_token_account.token_amount.decimals;

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

                let entry = accounts_by_token
                    .entry((token, decimals))
                    .or_insert_with(HashMap::new);
                entry.insert(token_account, (token_amount, frozen, close_authority));
            }
        }
    }

    let mut results = vec![];
    for ((token_pubkey, decimals), accounts) in accounts_by_token.into_iter() {
        println_display(config, format!("Processing token: {}", token_pubkey));

        let token = token_client_from_config(config, &token_pubkey, Some(decimals))?;
        let total_balance: u64 = accounts.values().map(|account| account.0).sum();

        let associated_token_account = token.get_associated_token_address(&owner);
        if !accounts.contains_key(&associated_token_account) && total_balance > 0 {
            token.create_associated_token_account(&owner).await?;
        }

        for (address, (amount, frozen, close_authority)) in accounts {
            let is_associated = address == associated_token_account;

            // only close the associated account if --close-empty-associated-accounts is
            // provided
            if is_associated && !close_empty_associated_accounts {
                continue;
            }

            // never close the associated account if *any* account carries a balance
            if is_associated && total_balance > 0 {
                continue;
            }

            // dont attempt to close frozen accounts
            if frozen {
                continue;
            }

            if is_associated {
                println!("Closing associated account {}", address);
            }

            // this logic is quite fiendish, but its more readable this way than if/else
            let maybe_res = match (close_authority == owner, is_associated, amount == 0) {
                // owner authority, associated or auxiliary, empty -> close
                (true, _, true) => Some(
                    token
                        .close_account(&address, &owner, &owner, &bulk_signers)
                        .await,
                ),
                // owner authority, auxiliary, nonempty -> empty and close
                (true, false, false) => Some(
                    token
                        .empty_and_close_account(
                            &address,
                            &owner,
                            &associated_token_account,
                            &owner,
                            &bulk_signers,
                        )
                        .await,
                ),
                // separate authority, auxiliary, nonempty -> transfer
                (false, false, false) => Some(
                    token
                        .transfer(
                            &address,
                            &associated_token_account,
                            &owner,
                            amount,
                            &bulk_signers,
                        )
                        .await,
                ),
                // separate authority, associated or auxiliary, empty -> print warning
                (false, _, true) => {
                    println_display(
                        config,
                        format!(
                            "Note: skipping {} due to separate close authority {}; \
                             revoke authority and rerun gc, or rerun gc with --owner",
                            address, close_authority
                        ),
                    );
                    None
                }
                // anything else, including a nonempty associated account -> unreachable
                (_, _, _) => unreachable!(),
            };

            if let Some(res) = maybe_res {
                let tx_return = finish_tx(config, &res?, false).await?;

                results.push(match tx_return {
                    TransactionReturnData::CliSignature(signature) => {
                        config.output_format.formatted_string(&signature)
                    }
                    TransactionReturnData::CliSignOnlyData(sign_only_data) => {
                        config.output_format.formatted_string(&sign_only_data)
                    }
                });
            };
        }
    }

    Ok(results.join(""))
}

async fn command_sync_native(config: &Config<'_>, native_account_address: Pubkey) -> CommandResult {
    let token = native_token_client_from_config(config)?;

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

async fn command_withdraw_excess_lamports(
    config: &Config<'_>,
    source_account: Pubkey,
    destination_account: Pubkey,
    authority: Pubkey,
    bulk_signers: Vec<Arc<dyn Signer>>,
) -> CommandResult {
    // default is safe here because withdraw_excess_lamports doesn't use it
    let token = token_client_from_config(config, &Pubkey::default(), None)?;
    println_display(
        config,
        format!(
            "Withdrawing excess lamports\n  Sender: {}\n  Destination: {}",
            source_account, destination_account
        ),
    );

    let res = token
        .withdraw_excess_lamports(
            &source_account,
            &destination_account,
            &authority,
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

// both enables and disables required transfer memos, via enable_memos bool
async fn command_required_transfer_memos(
    config: &Config<'_>,
    token_account_address: Pubkey,
    owner: Pubkey,
    bulk_signers: BulkSigners,
    enable_memos: bool,
) -> CommandResult {
    if config.sign_only {
        panic!("Config can not be sign-only for enabling/disabling required transfer memos.");
    }

    let account = config.get_account_checked(&token_account_address).await?;
    let current_account_len = account.data.len();

    let state_with_extension = StateWithExtensionsOwned::<Account>::unpack(account.data)?;
    let token = token_client_from_config(config, &state_with_extension.base.mint, None)?;

    // Reallocation (if needed)
    let mut existing_extensions: Vec<ExtensionType> = state_with_extension.get_extension_types()?;
    if existing_extensions.contains(&ExtensionType::MemoTransfer) {
        let extension_state = state_with_extension
            .get_extension::<MemoTransfer>()?
            .require_incoming_transfer_memos
            .into();

        if extension_state == enable_memos {
            return Ok(format!(
                "Required transfer memos were already {}",
                if extension_state {
                    "enabled"
                } else {
                    "disabled"
                }
            ));
        }
    } else {
        existing_extensions.push(ExtensionType::MemoTransfer);
        let needed_account_len =
            ExtensionType::try_calculate_account_len::<Account>(&existing_extensions)?;
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

// both enables and disables cpi guard, via enable_guard bool
async fn command_cpi_guard(
    config: &Config<'_>,
    token_account_address: Pubkey,
    owner: Pubkey,
    bulk_signers: BulkSigners,
    enable_guard: bool,
) -> CommandResult {
    if config.sign_only {
        panic!("Config can not be sign-only for enabling/disabling required transfer memos.");
    }

    let account = config.get_account_checked(&token_account_address).await?;
    let current_account_len = account.data.len();

    let state_with_extension = StateWithExtensionsOwned::<Account>::unpack(account.data)?;
    let token = token_client_from_config(config, &state_with_extension.base.mint, None)?;

    // reallocation (if needed)
    let mut existing_extensions: Vec<ExtensionType> = state_with_extension.get_extension_types()?;
    if existing_extensions.contains(&ExtensionType::CpiGuard) {
        let extension_state = state_with_extension
            .get_extension::<CpiGuard>()?
            .lock_cpi
            .into();

        if extension_state == enable_guard {
            return Ok(format!(
                "CPI Guard was already {}",
                if extension_state {
                    "enabled"
                } else {
                    "disabled"
                }
            ));
        }
    } else {
        existing_extensions.push(ExtensionType::CpiGuard);
        let required_account_len =
            ExtensionType::try_calculate_account_len::<Account>(&existing_extensions)?;
        if required_account_len > current_account_len {
            token
                .reallocate(
                    &token_account_address,
                    &owner,
                    &[ExtensionType::CpiGuard],
                    &bulk_signers,
                )
                .await?;
        }
    }

    let res = if enable_guard {
        token
            .enable_cpi_guard(&token_account_address, &owner, &bulk_signers)
            .await
    } else {
        token
            .disable_cpi_guard(&token_account_address, &owner, &bulk_signers)
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

async fn command_update_pointer_address(
    config: &Config<'_>,
    token_pubkey: Pubkey,
    authority: Pubkey,
    new_address: Option<Pubkey>,
    bulk_signers: BulkSigners,
    pointer: Pointer,
) -> CommandResult {
    if config.sign_only {
        panic!(
            "Config can not be sign-only for updating {} pointer address.",
            pointer
        );
    }

    let token = token_client_from_config(config, &token_pubkey, None)?;
    let res = match pointer {
        Pointer::Metadata => {
            token
                .update_metadata_address(&authority, new_address, &bulk_signers)
                .await
        }
        Pointer::Group => {
            token
                .update_group_address(&authority, new_address, &bulk_signers)
                .await
        }
        Pointer::GroupMember => {
            token
                .update_group_member_address(&authority, new_address, &bulk_signers)
                .await
        }
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

async fn command_update_default_account_state(
    config: &Config<'_>,
    token_pubkey: Pubkey,
    freeze_authority: Pubkey,
    new_default_state: AccountState,
    bulk_signers: BulkSigners,
) -> CommandResult {
    if !config.sign_only {
        let mint_account = config.get_account_checked(&token_pubkey).await?;

        let mint_state = StateWithExtensionsOwned::<Mint>::unpack(mint_account.data)
            .map_err(|_| format!("Could not deserialize token mint {}", token_pubkey))?;
        match mint_state.base.freeze_authority {
            COption::None => {
                return Err(format!("Mint {} has no freeze authority.", token_pubkey).into())
            }
            COption::Some(mint_freeze_authority) => {
                if mint_freeze_authority != freeze_authority {
                    return Err(format!(
                        "Mint {} has a freeze authority {}, {} provided",
                        token_pubkey, mint_freeze_authority, freeze_authority
                    )
                    .into());
                }
            }
        }

        if let Ok(default_account_state) = mint_state.get_extension::<DefaultAccountState>() {
            if default_account_state.state == u8::from(new_default_state) {
                let state_string = match new_default_state {
                    AccountState::Frozen => "frozen",
                    AccountState::Initialized => "initialized",
                    _ => unreachable!(),
                };
                return Err(format!(
                    "Mint {} already has default account state {}",
                    token_pubkey, state_string
                )
                .into());
            }
        } else {
            return Err(format!(
                "Mint {} does not support default account states",
                token_pubkey
            )
            .into());
        }
    }

    let token = token_client_from_config(config, &token_pubkey, None)?;
    let res = token
        .set_default_account_state(&freeze_authority, &new_default_state, &bulk_signers)
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

async fn command_withdraw_withheld_tokens(
    config: &Config<'_>,
    destination_token_account: Pubkey,
    source_token_accounts: Vec<Pubkey>,
    authority: Pubkey,
    include_mint: bool,
    bulk_signers: BulkSigners,
) -> CommandResult {
    if config.sign_only {
        panic!("Config can not be sign-only for withdrawing withheld tokens.");
    }
    let destination_account = config
        .get_account_checked(&destination_token_account)
        .await?;
    let destination_state = StateWithExtensionsOwned::<Account>::unpack(destination_account.data)
        .map_err(|_| {
        format!(
            "Could not deserialize token account {}",
            destination_token_account
        )
    })?;
    let token_pubkey = destination_state.base.mint;
    destination_state
        .get_extension::<TransferFeeAmount>()
        .map_err(|_| format!("Token mint {} has no transfer fee configured", token_pubkey))?;

    let token = token_client_from_config(config, &token_pubkey, None)?;
    let mut results = vec![];
    if include_mint {
        let res = token
            .withdraw_withheld_tokens_from_mint(
                &destination_token_account,
                &authority,
                &bulk_signers,
            )
            .await;
        let tx_return = finish_tx(config, &res?, false).await?;
        results.push(match tx_return {
            TransactionReturnData::CliSignature(signature) => {
                config.output_format.formatted_string(&signature)
            }
            TransactionReturnData::CliSignOnlyData(sign_only_data) => {
                config.output_format.formatted_string(&sign_only_data)
            }
        });
    }

    let source_refs = source_token_accounts.iter().collect::<Vec<_>>();
    // this can be tweaked better, but keep it simple for now
    const MAX_WITHDRAWAL_ACCOUNTS: usize = 25;
    for sources in source_refs.chunks(MAX_WITHDRAWAL_ACCOUNTS) {
        let res = token
            .withdraw_withheld_tokens_from_accounts(
                &destination_token_account,
                &authority,
                sources,
                &bulk_signers,
            )
            .await;
        let tx_return = finish_tx(config, &res?, false).await?;
        results.push(match tx_return {
            TransactionReturnData::CliSignature(signature) => {
                config.output_format.formatted_string(&signature)
            }
            TransactionReturnData::CliSignOnlyData(sign_only_data) => {
                config.output_format.formatted_string(&sign_only_data)
            }
        });
    }

    Ok(results.join(""))
}

async fn command_update_confidential_transfer_settings(
    config: &Config<'_>,
    token_pubkey: Pubkey,
    authority: Pubkey,
    auto_approve: Option<bool>,
    auditor_pubkey: Option<ElGamalPubkeyOrNone>,
    bulk_signers: Vec<Arc<dyn Signer>>,
) -> CommandResult {
    let (new_auto_approve, new_auditor_pubkey) = if !config.sign_only {
        let confidential_transfer_account = config.get_account_checked(&token_pubkey).await?;

        let mint_state =
            StateWithExtensionsOwned::<Mint>::unpack(confidential_transfer_account.data)
                .map_err(|_| format!("Could not deserialize token mint {}", token_pubkey))?;

        if let Ok(confidential_transfer_mint) =
            mint_state.get_extension::<ConfidentialTransferMint>()
        {
            let expected_authority = Option::<Pubkey>::from(confidential_transfer_mint.authority);

            if expected_authority != Some(authority) {
                return Err(format!(
                    "Mint {} has confidential transfer authority {}, but {} was provided",
                    token_pubkey,
                    expected_authority
                        .map(|pubkey| pubkey.to_string())
                        .unwrap_or_else(|| "disabled".to_string()),
                    authority
                )
                .into());
            }

            let new_auto_approve = if let Some(auto_approve) = auto_approve {
                auto_approve
            } else {
                bool::from(confidential_transfer_mint.auto_approve_new_accounts)
            };

            let new_auditor_pubkey = if let Some(auditor_pubkey) = auditor_pubkey {
                auditor_pubkey.into()
            } else {
                Option::<PodElGamalPubkey>::from(confidential_transfer_mint.auditor_elgamal_pubkey)
            };

            (new_auto_approve, new_auditor_pubkey)
        } else {
            return Err(format!(
                "Mint {} does not support confidential transfers",
                token_pubkey
            )
            .into());
        }
    } else {
        let new_auto_approve = auto_approve.expect("The approve policy must be provided");
        let new_auditor_pubkey = auditor_pubkey
            .expect("The auditor encryption pubkey must be provided")
            .into();

        (new_auto_approve, new_auditor_pubkey)
    };

    println_display(
        config,
        format!(
            "Updating confidential transfer settings for {}:",
            token_pubkey,
        ),
    );

    if auto_approve.is_some() {
        println_display(
            config,
            format!(
                "  approve policy set to {}",
                if new_auto_approve { "auto" } else { "manual" }
            ),
        );
    }

    if auditor_pubkey.is_some() {
        if let Some(new_auditor_pubkey) = new_auditor_pubkey {
            println_display(
                config,
                format!("  auditor encryption pubkey set to {}", new_auditor_pubkey,),
            );
        } else {
            println_display(config, "  auditability disabled".to_string())
        }
    }

    let token = token_client_from_config(config, &token_pubkey, None)?;
    let res = token
        .confidential_transfer_update_mint(
            &authority,
            new_auto_approve,
            new_auditor_pubkey,
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

#[allow(clippy::too_many_arguments)]
async fn command_configure_confidential_transfer_account(
    config: &Config<'_>,
    maybe_token: Option<Pubkey>,
    owner: Pubkey,
    maybe_account: Option<Pubkey>,
    maximum_credit_counter: Option<u64>,
    elgamal_keypair: &ElGamalKeypair,
    aes_key: &AeKey,
    bulk_signers: BulkSigners,
) -> CommandResult {
    if config.sign_only {
        panic!("Sign-only is not yet supported.");
    }

    let token_account_address = if let Some(account) = maybe_account {
        account
    } else {
        let token_pubkey =
            maybe_token.expect("Either a valid token or account address must be provided");
        let token = token_client_from_config(config, &token_pubkey, None)?;
        token.get_associated_token_address(&owner)
    };

    let account = config.get_account_checked(&token_account_address).await?;
    let current_account_len = account.data.len();

    let state_with_extension = StateWithExtensionsOwned::<Account>::unpack(account.data)?;
    let token = token_client_from_config(config, &state_with_extension.base.mint, None)?;

    // Reallocation (if needed)
    let mut existing_extensions: Vec<ExtensionType> = state_with_extension.get_extension_types()?;
    if !existing_extensions.contains(&ExtensionType::ConfidentialTransferAccount) {
        let mut extra_extensions = vec![ExtensionType::ConfidentialTransferAccount];
        if existing_extensions.contains(&ExtensionType::TransferFeeAmount) {
            extra_extensions.push(ExtensionType::ConfidentialTransferFeeAmount);
        }
        existing_extensions.extend_from_slice(&extra_extensions);
        let needed_account_len =
            ExtensionType::try_calculate_account_len::<Account>(&existing_extensions)?;
        if needed_account_len > current_account_len {
            token
                .reallocate(
                    &token_account_address,
                    &owner,
                    &extra_extensions,
                    &bulk_signers,
                )
                .await?;
        }
    }

    let res = token
        .confidential_transfer_configure_token_account(
            &token_account_address,
            &owner,
            None,
            maximum_credit_counter,
            elgamal_keypair,
            aes_key,
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

async fn command_enable_disable_confidential_transfers(
    config: &Config<'_>,
    maybe_token: Option<Pubkey>,
    owner: Pubkey,
    maybe_account: Option<Pubkey>,
    bulk_signers: BulkSigners,
    allow_confidential_credits: Option<bool>,
    allow_non_confidential_credits: Option<bool>,
) -> CommandResult {
    if config.sign_only {
        panic!("Sign-only is not yet supported.");
    }

    let token_account_address = if let Some(account) = maybe_account {
        account
    } else {
        let token_pubkey =
            maybe_token.expect("Either a valid token or account address must be provided");
        let token = token_client_from_config(config, &token_pubkey, None)?;
        token.get_associated_token_address(&owner)
    };

    let account = config.get_account_checked(&token_account_address).await?;

    let state_with_extension = StateWithExtensionsOwned::<Account>::unpack(account.data)?;
    let token = token_client_from_config(config, &state_with_extension.base.mint, None)?;

    let existing_extensions: Vec<ExtensionType> = state_with_extension.get_extension_types()?;
    if !existing_extensions.contains(&ExtensionType::ConfidentialTransferAccount) {
        panic!(
            "Confidential transfer is not yet configured for this account. \
        Use `configure-confidential-transfer-account` command instead."
        );
    }

    let res = if let Some(allow_confidential_credits) = allow_confidential_credits {
        let extension_state = state_with_extension
            .get_extension::<ConfidentialTransferAccount>()?
            .allow_confidential_credits
            .into();

        if extension_state == allow_confidential_credits {
            return Ok(format!(
                "Confidential transfers are already {}",
                if extension_state {
                    "enabled"
                } else {
                    "disabled"
                }
            ));
        }

        if allow_confidential_credits {
            token
                .confidential_transfer_enable_confidential_credits(
                    &token_account_address,
                    &owner,
                    &bulk_signers,
                )
                .await
        } else {
            token
                .confidential_transfer_disable_confidential_credits(
                    &token_account_address,
                    &owner,
                    &bulk_signers,
                )
                .await
        }
    } else {
        let allow_non_confidential_credits =
            allow_non_confidential_credits.expect("Nothing to be done");
        let extension_state = state_with_extension
            .get_extension::<ConfidentialTransferAccount>()?
            .allow_non_confidential_credits
            .into();

        if extension_state == allow_non_confidential_credits {
            return Ok(format!(
                "Non-confidential transfers are already {}",
                if extension_state {
                    "enabled"
                } else {
                    "disabled"
                }
            ));
        }

        if allow_non_confidential_credits {
            token
                .confidential_transfer_enable_non_confidential_credits(
                    &token_account_address,
                    &owner,
                    &bulk_signers,
                )
                .await
        } else {
            token
                .confidential_transfer_disable_non_confidential_credits(
                    &token_account_address,
                    &owner,
                    &bulk_signers,
                )
                .await
        }
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
#[derive(PartialEq, Eq)]
enum ConfidentialInstructionType {
    Deposit,
    Withdraw,
}

#[allow(clippy::too_many_arguments)]
async fn command_deposit_withdraw_confidential_tokens(
    config: &Config<'_>,
    token_pubkey: Pubkey,
    owner: Pubkey,
    maybe_account: Option<Pubkey>,
    bulk_signers: BulkSigners,
    ui_amount: Amount,
    mint_decimals: Option<u8>,
    instruction_type: ConfidentialInstructionType,
    elgamal_keypair: Option<&ElGamalKeypair>,
    aes_key: Option<&AeKey>,
) -> CommandResult {
    if config.sign_only {
        panic!("Sign-only is not yet supported.");
    }

    // check if mint decimals provided is consistent
    let mint_info = config.get_mint_info(&token_pubkey, mint_decimals).await?;

    if !config.sign_only && mint_decimals.is_some() && mint_decimals != Some(mint_info.decimals) {
        return Err(format!(
            "Decimals {} was provided, but actual value is {}",
            mint_decimals.unwrap(),
            mint_info.decimals
        )
        .into());
    }

    let decimals = if let Some(decimals) = mint_decimals {
        decimals
    } else {
        mint_info.decimals
    };

    // derive ATA if account address not provided
    let token_account_address = if let Some(account) = maybe_account {
        account
    } else {
        let token = token_client_from_config(config, &token_pubkey, Some(decimals))?;
        token.get_associated_token_address(&owner)
    };

    let account = config.get_account_checked(&token_account_address).await?;

    let state_with_extension = StateWithExtensionsOwned::<Account>::unpack(account.data)?;
    let token = token_client_from_config(config, &state_with_extension.base.mint, None)?;

    // the amount the user wants to deposit or withdraw, as a u64
    let amount = match ui_amount {
        Amount::Raw(ui_amount) => ui_amount,
        Amount::Decimal(ui_amount) => spl_token::ui_amount_to_amount(ui_amount, mint_info.decimals),
        Amount::All => {
            if config.sign_only {
                return Err("Use of ALL keyword to burn tokens requires online signing"
                    .to_string()
                    .into());
            }
            if instruction_type == ConfidentialInstructionType::Withdraw {
                return Err("ALL keyword is not currently supported for withdraw"
                    .to_string()
                    .into());
            }
            state_with_extension.base.amount
        }
    };

    match instruction_type {
        ConfidentialInstructionType::Deposit => {
            println_display(
                config,
                format!(
                    "Depositing {} confidential tokens",
                    spl_token::amount_to_ui_amount(amount, mint_info.decimals),
                ),
            );
            let current_balance = state_with_extension.base.amount;
            if amount > current_balance {
                return Err(format!(
                    "Error: Insufficient funds, current balance is {}",
                    spl_token_2022::amount_to_ui_amount_string_trimmed(
                        current_balance,
                        mint_info.decimals
                    )
                )
                .into());
            }
        }
        ConfidentialInstructionType::Withdraw => {
            println_display(
                config,
                format!(
                    "Withdrawing {} confidential tokens",
                    spl_token::amount_to_ui_amount(amount, mint_info.decimals)
                ),
            );
        }
    }

    let res = match instruction_type {
        ConfidentialInstructionType::Deposit => {
            token
                .confidential_transfer_deposit(
                    &token_account_address,
                    &owner,
                    amount,
                    decimals,
                    &bulk_signers,
                )
                .await?
        }
        ConfidentialInstructionType::Withdraw => {
            let elgamal_keypair = elgamal_keypair.expect("ElGamal keypair must be provided");
            let aes_key = aes_key.expect("AES key must be provided");

            let extension_state =
                state_with_extension.get_extension::<ConfidentialTransferAccount>()?;
            let withdraw_account_info = WithdrawAccountInfo::new(extension_state);

            let context_state_authority = config.fee_payer()?;
            let equality_proof_context_state_keypair = Keypair::new();
            let equality_proof_context_state_pubkey = equality_proof_context_state_keypair.pubkey();
            let range_proof_context_state_keypair = Keypair::new();
            let range_proof_context_state_pubkey = range_proof_context_state_keypair.pubkey();

            let WithdrawProofData {
                equality_proof_data,
                range_proof_data,
            } = withdraw_account_info.generate_proof_data(amount, elgamal_keypair, aes_key)?;

            // set up context state accounts
            let context_state_authority_pubkey = context_state_authority.pubkey();
            let create_equality_proof_signer = &[&equality_proof_context_state_keypair];
            let create_range_proof_signer = &[&range_proof_context_state_keypair];

            let _ = try_join!(
                token.confidential_transfer_create_context_state_account(
                    &equality_proof_context_state_pubkey,
                    &context_state_authority_pubkey,
                    &equality_proof_data,
                    false,
                    create_equality_proof_signer
                ),
                token.confidential_transfer_create_context_state_account(
                    &range_proof_context_state_pubkey,
                    &context_state_authority_pubkey,
                    &range_proof_data,
                    true,
                    create_range_proof_signer,
                )
            )?;

            // do the withdrawal
            let withdraw_result = token
                .confidential_transfer_withdraw(
                    &token_account_address,
                    &owner,
                    Some(&ProofAccount::ContextAccount(
                        equality_proof_context_state_pubkey,
                    )),
                    Some(&ProofAccount::ContextAccount(
                        range_proof_context_state_pubkey,
                    )),
                    amount,
                    decimals,
                    Some(withdraw_account_info),
                    elgamal_keypair,
                    aes_key,
                    &bulk_signers,
                )
                .await?;

            // close context state account
            let close_context_state_signer = &[&context_state_authority];
            let _ = try_join!(
                token.confidential_transfer_close_context_state_account(
                    &equality_proof_context_state_pubkey,
                    &token_account_address,
                    &context_state_authority_pubkey,
                    close_context_state_signer
                ),
                token.confidential_transfer_close_context_state_account(
                    &range_proof_context_state_pubkey,
                    &token_account_address,
                    &context_state_authority_pubkey,
                    close_context_state_signer
                )
            )?;

            withdraw_result
        }
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

#[allow(clippy::too_many_arguments)]
async fn command_apply_pending_balance(
    config: &Config<'_>,
    maybe_token: Option<Pubkey>,
    owner: Pubkey,
    maybe_account: Option<Pubkey>,
    bulk_signers: BulkSigners,
    elgamal_keypair: &ElGamalKeypair,
    aes_key: &AeKey,
) -> CommandResult {
    if config.sign_only {
        panic!("Sign-only is not yet supported.");
    }

    // derive ATA if account address not provided
    let token_account_address = if let Some(account) = maybe_account {
        account
    } else {
        let token_pubkey =
            maybe_token.expect("Either a valid token or account address must be provided");
        let token = token_client_from_config(config, &token_pubkey, None)?;
        token.get_associated_token_address(&owner)
    };

    let account = config.get_account_checked(&token_account_address).await?;

    let state_with_extension = StateWithExtensionsOwned::<Account>::unpack(account.data)?;
    let token = token_client_from_config(config, &state_with_extension.base.mint, None)?;

    let extension_state = state_with_extension.get_extension::<ConfidentialTransferAccount>()?;
    let account_info = ApplyPendingBalanceAccountInfo::new(extension_state);

    let res = token
        .confidential_transfer_apply_pending_balance(
            &token_account_address,
            &owner,
            Some(account_info),
            elgamal_keypair.secret(),
            aes_key,
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

struct ConfidentialTransferArgs {
    sender_elgamal_keypair: ElGamalKeypair,
    sender_aes_key: AeKey,
    recipient_elgamal_pubkey: Option<PodElGamalPubkey>,
    auditor_elgamal_pubkey: Option<PodElGamalPubkey>,
}

pub async fn process_command<'a>(
    sub_command: &CommandName,
    sub_matches: &ArgMatches,
    config: &Config<'a>,
    mut wallet_manager: Option<Rc<RemoteWalletManager>>,
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
            let decimals = *arg_matches.get_one::<u8>("decimals").unwrap();
            let mint_authority =
                config.pubkey_or_default(arg_matches, "mint_authority", &mut wallet_manager)?;
            let memo = value_t!(arg_matches, "memo", String).ok();
            let rate_bps = value_t!(arg_matches, "interest_rate", i16).ok();
            let metadata_address =
                SignerSource::try_get_pubkey(arg_matches, "metadata_address", &mut wallet_manager)
                    .unwrap();
            let group_address =
                SignerSource::try_get_pubkey(arg_matches, "group_address", &mut wallet_manager)
                    .unwrap();
            let member_address =
                SignerSource::try_get_pubkey(arg_matches, "member_address", &mut wallet_manager)
                    .unwrap();

            let transfer_fee = arg_matches.values_of("transfer_fee").map(|mut v| {
                println_display(config,"transfer-fee has been deprecated and will be removed in a future release. Please specify --transfer-fee-basis-points and --transfer-fee-maximum-fee with a UI amount".to_string());
                (
                    v.next()
                        .unwrap()
                        .parse::<u16>()
                        .unwrap_or_else(print_error_and_exit),
                    v.next()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap_or_else(print_error_and_exit),
                )
            });

            let transfer_fee_basis_point = arg_matches.get_one::<u16>("transfer_fee_basis_points");
            let transfer_fee_maximum_fee = arg_matches
                .get_one::<Amount>("transfer_fee_maximum_fee")
                .map(|v| amount_to_raw_amount(*v, decimals, None, "MAXIMUM_FEE"));
            let transfer_fee = transfer_fee_basis_point
                .map(|v| (*v, transfer_fee_maximum_fee.unwrap()))
                .or(transfer_fee);

            let (token_signer, token) =
                get_signer(arg_matches, "token_keypair", &mut wallet_manager)
                    .unwrap_or_else(new_throwaway_signer);
            push_signer_with_dedup(token_signer, &mut bulk_signers);
            let default_account_state =
                arg_matches
                    .value_of("default_account_state")
                    .map(|s| match s {
                        "initialized" => AccountState::Initialized,
                        "frozen" => AccountState::Frozen,
                        _ => unreachable!(),
                    });
            let transfer_hook_program_id =
                SignerSource::try_get_pubkey(arg_matches, "transfer_hook", &mut wallet_manager)
                    .unwrap();

            let confidential_transfer_auto_approve = arg_matches
                .value_of("enable_confidential_transfers")
                .map(|b| b == "auto");

            command_create_token(
                config,
                decimals,
                token,
                mint_authority,
                arg_matches.is_present("enable_freeze"),
                arg_matches.is_present("enable_close"),
                arg_matches.is_present("enable_non_transferable"),
                arg_matches.is_present("enable_permanent_delegate"),
                memo,
                metadata_address,
                group_address,
                member_address,
                rate_bps,
                default_account_state,
                transfer_fee,
                confidential_transfer_auto_approve,
                transfer_hook_program_id,
                arg_matches.is_present("enable_metadata"),
                arg_matches.is_present("enable_group"),
                arg_matches.is_present("enable_member"),
                bulk_signers,
            )
            .await
        }
        (CommandName::SetInterestRate, arg_matches) => {
            let token_pubkey =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
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
        (CommandName::SetTransferHook, arg_matches) => {
            let token_pubkey =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let new_program_id =
                SignerSource::try_get_pubkey(arg_matches, "new_program_id", &mut wallet_manager)
                    .unwrap();
            let (authority_signer, authority_pubkey) =
                config.signer_or_default(arg_matches, "authority", &mut wallet_manager);
            let bulk_signers = vec![authority_signer];

            command_set_transfer_hook_program(
                config,
                token_pubkey,
                authority_pubkey,
                new_program_id,
                bulk_signers,
            )
            .await
        }
        (CommandName::InitializeMetadata, arg_matches) => {
            let token_pubkey =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let name = arg_matches.value_of("name").unwrap().to_string();
            let symbol = arg_matches.value_of("symbol").unwrap().to_string();
            let uri = arg_matches.value_of("uri").unwrap().to_string();
            let (mint_authority_signer, mint_authority) =
                config.signer_or_default(arg_matches, "mint_authority", &mut wallet_manager);
            let bulk_signers = vec![mint_authority_signer];
            let update_authority =
                config.pubkey_or_default(arg_matches, "update_authority", &mut wallet_manager)?;

            command_initialize_metadata(
                config,
                token_pubkey,
                update_authority,
                mint_authority,
                name,
                symbol,
                uri,
                bulk_signers,
            )
            .await
        }
        (CommandName::UpdateMetadata, arg_matches) => {
            let token_pubkey =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let (authority_signer, authority) =
                config.signer_or_default(arg_matches, "authority", &mut wallet_manager);
            let field = arg_matches.value_of("field").unwrap();
            let field = match field.to_lowercase().as_str() {
                "name" => Field::Name,
                "symbol" => Field::Symbol,
                "uri" => Field::Uri,
                _ => Field::Key(field.to_string()),
            };
            let value = arg_matches.value_of("value").map(|v| v.to_string());
            let transfer_lamports = arg_matches
                .get_one::<u64>(TRANSFER_LAMPORTS_ARG.name)
                .copied();
            let bulk_signers = vec![authority_signer];

            command_update_metadata(
                config,
                token_pubkey,
                authority,
                field,
                value,
                transfer_lamports,
                bulk_signers,
            )
            .await
        }
        (CommandName::InitializeGroup, arg_matches) => {
            let token_pubkey =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let max_size = *arg_matches.get_one::<u64>("max_size").unwrap();
            let (mint_authority_signer, mint_authority) =
                config.signer_or_default(arg_matches, "mint_authority", &mut wallet_manager);
            let update_authority =
                config.pubkey_or_default(arg_matches, "update_authority", &mut wallet_manager)?;
            let bulk_signers = vec![mint_authority_signer];

            command_initialize_group(
                config,
                token_pubkey,
                mint_authority,
                update_authority,
                max_size,
                bulk_signers,
            )
            .await
        }
        (CommandName::UpdateGroupMaxSize, arg_matches) => {
            let token_pubkey =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let new_max_size = *arg_matches.get_one::<u64>("new_max_size").unwrap();
            let (update_authority_signer, update_authority) =
                config.signer_or_default(arg_matches, "update_authority", &mut wallet_manager);
            let bulk_signers = vec![update_authority_signer];

            command_update_group_max_size(
                config,
                token_pubkey,
                update_authority,
                new_max_size,
                bulk_signers,
            )
            .await
        }
        (CommandName::InitializeMember, arg_matches) => {
            let member_token_pubkey =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let group_token_pubkey =
                SignerSource::try_get_pubkey(arg_matches, "group_token", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let (mint_authority_signer, mint_authority) =
                config.signer_or_default(arg_matches, "mint_authority", &mut wallet_manager);
            let (group_update_authority_signer, group_update_authority) = config.signer_or_default(
                arg_matches,
                "group_update_authority",
                &mut wallet_manager,
            );
            let mut bulk_signers = vec![mint_authority_signer];
            push_signer_with_dedup(group_update_authority_signer, &mut bulk_signers);

            command_initialize_member(
                config,
                member_token_pubkey,
                mint_authority,
                group_token_pubkey,
                group_update_authority,
                bulk_signers,
            )
            .await
        }
        (CommandName::CreateAccount, arg_matches) => {
            let token = SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();

            // No need to add a signer when creating an associated token account
            let account = get_signer(arg_matches, "account_keypair", &mut wallet_manager).map(
                |(signer, account)| {
                    push_signer_with_dedup(signer, &mut bulk_signers);
                    account
                },
            );

            let owner = config.pubkey_or_default(arg_matches, "owner", &mut wallet_manager)?;
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
            let minimum_signers = arg_matches
                .get_one("minimum_signers")
                .map(|v: &String| v.parse::<u8>().unwrap())
                .unwrap();
            let multisig_members =
                SignerSource::try_get_pubkeys(arg_matches, "multisig_member", &mut wallet_manager)
                    .unwrap_or_else(print_error_and_exit)
                    .unwrap_or_default();
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
            let address = SignerSource::try_get_pubkey(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let authority_type = arg_matches.value_of("authority_type").unwrap();
            let authority_type = CliAuthorityType::from_str(authority_type)?;

            let (authority_signer, authority) =
                config.signer_or_default(arg_matches, "authority", &mut wallet_manager);
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(authority_signer, &mut bulk_signers);
            }

            let new_authority =
                SignerSource::try_get_pubkey(arg_matches, "new_authority", &mut wallet_manager)
                    .unwrap();
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
            let token = SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let amount = *arg_matches.get_one::<Amount>("amount").unwrap();
            let recipient =
                SignerSource::try_get_pubkey(arg_matches, "recipient", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let sender =
                SignerSource::try_get_pubkey(arg_matches, "from", &mut wallet_manager).unwrap();

            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);

            let confidential_transfer_args = if arg_matches.is_present("confidential") {
                // Deriving ElGamal and AES key from signer. Custom ElGamal and AES keys will be
                // supported in the future once upgrading to clap-v3.
                //
                // NOTE:: Seed bytes are hardcoded to be empty bytes for now. They will be
                // updated once custom ElGamal and AES keys are supported.
                let sender_elgamal_keypair =
                    ElGamalKeypair::new_from_signer(&*owner_signer, b"").unwrap();
                let sender_aes_key = AeKey::new_from_signer(&*owner_signer, b"").unwrap();

                // Sign-only mode is not yet supported for confidential transfers, so set
                // recipient and auditor ElGamal public to `None` by default.
                Some(ConfidentialTransferArgs {
                    sender_elgamal_keypair,
                    sender_aes_key,
                    recipient_elgamal_pubkey: None,
                    auditor_elgamal_pubkey: None,
                })
            } else {
                None
            };

            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(owner_signer, &mut bulk_signers);
            }

            let mint_decimals = arg_matches.get_one::<u8>(MINT_DECIMALS_ARG.name).copied();
            let fund_recipient = arg_matches.is_present("fund_recipient");
            let allow_unfunded_recipient = arg_matches.is_present("allow_empty_recipient")
                || arg_matches.is_present("allow_unfunded_recipient");

            let recipient_is_ata_owner = arg_matches.is_present("recipient_is_ata_owner");
            let no_recipient_is_ata_owner =
                arg_matches.is_present("no_recipient_is_ata_owner") || !recipient_is_ata_owner;
            if recipient_is_ata_owner {
                println_display(config, "recipient-is-ata-owner is now the default behavior. The option has been deprecated and will be removed in a future release.".to_string());
            }
            let use_unchecked_instruction = arg_matches.is_present("use_unchecked_instruction");
            let expected_fee = arg_matches.get_one::<Amount>("expected_fee").copied();
            let memo = value_t!(arg_matches, "memo", String).ok();
            let transfer_hook_accounts = arg_matches
                .get_many::<TransferHookAccount>("transfer_hook_account")
                .map(|v| {
                    v.into_iter()
                        .map(|account| account.create_account_meta())
                        .collect::<Vec<_>>()
                });

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
                no_recipient_is_ata_owner,
                use_unchecked_instruction,
                expected_fee,
                memo,
                bulk_signers,
                arg_matches.is_present("no_wait"),
                arg_matches.is_present("allow_non_system_account_recipient"),
                transfer_hook_accounts,
                confidential_transfer_args.as_ref(),
            )
            .await
        }
        (CommandName::Burn, arg_matches) => {
            let account = SignerSource::try_get_pubkey(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();

            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(owner_signer, &mut bulk_signers);
            }

            let amount = *arg_matches.get_one::<Amount>("amount").unwrap();
            let mint_address =
                pubkey_of_signer(arg_matches, MINT_ADDRESS_ARG.name, &mut wallet_manager).unwrap();
            let mint_decimals = arg_matches
                .get_one(MINT_DECIMALS_ARG.name)
                .map(|v: &String| v.parse::<u8>().unwrap());
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
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(mint_authority_signer, &mut bulk_signers);
            }

            let token = SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let amount = *arg_matches.get_one::<Amount>("amount").unwrap();
            let mint_decimals = arg_matches.get_one::<u8>(MINT_DECIMALS_ARG.name).copied();
            let mint_info = config.get_mint_info(&token, mint_decimals).await?;
            let recipient = if let Some(address) =
                SignerSource::try_get_pubkey(arg_matches, "recipient", &mut wallet_manager).unwrap()
            {
                address
            } else if let Some(address) =
                SignerSource::try_get_pubkey(arg_matches, "recipient_owner", &mut wallet_manager)
                    .unwrap()
            {
                get_associated_token_address_with_program_id(&address, &token, &config.program_id)
            } else {
                let owner = config.default_signer()?.pubkey();
                config.associated_token_address_for_token_and_program(
                    &mint_info.address,
                    &owner,
                    &mint_info.program_id,
                )?
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
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(freeze_authority_signer, &mut bulk_signers);
            }

            let account = SignerSource::try_get_pubkey(arg_matches, "account", &mut wallet_manager)
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
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(freeze_authority_signer, &mut bulk_signers);
            }

            let account = SignerSource::try_get_pubkey(arg_matches, "account", &mut wallet_manager)
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
            let amount = *arg_matches.get_one::<Amount>("amount").unwrap();
            let account = if arg_matches.is_present("create_aux_account") {
                let (signer, account) = new_throwaway_signer();
                bulk_signers.push(signer);
                Some(account)
            } else {
                // No need to add a signer when creating an associated token account
                None
            };

            let (wallet_signer, wallet_address) =
                config.signer_or_default(arg_matches, "wallet_keypair", &mut wallet_manager);
            push_signer_with_dedup(wallet_signer, &mut bulk_signers);

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
            push_signer_with_dedup(wallet_signer, &mut bulk_signers);

            let account =
                SignerSource::try_get_pubkey(arg_matches, "account", &mut wallet_manager).unwrap();
            command_unwrap(config, wallet_address, account, bulk_signers).await
        }
        (CommandName::Approve, arg_matches) => {
            let (owner_signer, owner_address) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(owner_signer, &mut bulk_signers);
            }

            let account = SignerSource::try_get_pubkey(arg_matches, "account", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let amount = *arg_matches.get_one::<Amount>("amount").unwrap();
            let delegate =
                SignerSource::try_get_pubkey(arg_matches, "delegate", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let mint_address = SignerSource::try_get_pubkey(
                arg_matches,
                MINT_ADDRESS_ARG.name,
                &mut wallet_manager,
            )
            .unwrap();
            let mint_decimals = arg_matches
                .get_one(MINT_DECIMALS_ARG.name)
                .map(|v: &String| v.parse::<u8>().unwrap());
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
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(owner_signer, &mut bulk_signers);
            }

            let account = SignerSource::try_get_pubkey(arg_matches, "account", &mut wallet_manager)
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
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(close_authority_signer, &mut bulk_signers);
            }

            let address = config
                .associated_token_address_or_override(arg_matches, "address", &mut wallet_manager)
                .await?;
            let recipient =
                config.pubkey_or_default(arg_matches, "recipient", &mut wallet_manager)?;
            command_close(config, address, close_authority, recipient, bulk_signers).await
        }
        (CommandName::CloseMint, arg_matches) => {
            let token = SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let (close_authority_signer, close_authority) =
                config.signer_or_default(arg_matches, "close_authority", &mut wallet_manager);
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(close_authority_signer, &mut bulk_signers);
            }
            let recipient =
                config.pubkey_or_default(arg_matches, "recipient", &mut wallet_manager)?;

            command_close_mint(config, token, close_authority, recipient, bulk_signers).await
        }
        (CommandName::Balance, arg_matches) => {
            let address = config
                .associated_token_address_or_override(arg_matches, "address", &mut wallet_manager)
                .await?;
            command_balance(config, address).await
        }
        (CommandName::Supply, arg_matches) => {
            let token = SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_supply(config, token).await
        }
        (CommandName::Accounts, arg_matches) => {
            let token =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager).unwrap();
            let owner = config.pubkey_or_default(arg_matches, "owner", &mut wallet_manager)?;
            let filter = if arg_matches.is_present("delegated") {
                AccountFilter::Delegated
            } else if arg_matches.is_present("externally_closeable") {
                AccountFilter::ExternallyCloseable
            } else {
                AccountFilter::All
            };

            command_accounts(
                config,
                token,
                owner,
                filter,
                arg_matches.is_present("addresses_only"),
            )
            .await
        }
        (CommandName::Address, arg_matches) => {
            let token =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager).unwrap();
            let owner = config.pubkey_or_default(arg_matches, "owner", &mut wallet_manager)?;
            command_address(config, token, owner).await
        }
        (CommandName::AccountInfo, arg_matches) => {
            let address = config
                .associated_token_address_or_override(arg_matches, "address", &mut wallet_manager)
                .await?;
            command_display(config, address).await
        }
        (CommandName::MultisigInfo, arg_matches) => {
            let address = SignerSource::try_get_pubkey(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_display(config, address).await
        }
        (CommandName::Display, arg_matches) => {
            let address = SignerSource::try_get_pubkey(arg_matches, "address", &mut wallet_manager)
                .unwrap()
                .unwrap();
            command_display(config, address).await
        }
        (CommandName::Gc, arg_matches) => {
            match config.output_format {
                OutputFormat::Json | OutputFormat::JsonCompact => {
                    eprintln!(
                        "`spl-token gc` does not support the `--output` parameter at this time"
                    );
                    exit(1);
                }
                _ => {}
            }

            let close_empty_associated_accounts =
                arg_matches.is_present("close_empty_associated_accounts");

            let (owner_signer, owner_address) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(owner_signer, &mut bulk_signers);
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
            let native_mint = *native_token_client_from_config(config)?.get_address();
            let address = config
                .associated_token_address_for_token_or_override(
                    arg_matches,
                    "address",
                    &mut wallet_manager,
                    Some(native_mint),
                )
                .await;
            command_sync_native(config, address?).await
        }
        (CommandName::EnableRequiredTransferMemos, arg_matches) => {
            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(owner_signer, &mut bulk_signers);
            }
            // Since account is required argument it will always be present
            let token_account =
                config.pubkey_or_default(arg_matches, "account", &mut wallet_manager)?;
            command_required_transfer_memos(config, token_account, owner, bulk_signers, true).await
        }
        (CommandName::DisableRequiredTransferMemos, arg_matches) => {
            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(owner_signer, &mut bulk_signers);
            }
            // Since account is required argument it will always be present
            let token_account =
                config.pubkey_or_default(arg_matches, "account", &mut wallet_manager)?;
            command_required_transfer_memos(config, token_account, owner, bulk_signers, false).await
        }
        (CommandName::EnableCpiGuard, arg_matches) => {
            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(owner_signer, &mut bulk_signers);
            }
            // Since account is required argument it will always be present
            let token_account =
                config.pubkey_or_default(arg_matches, "account", &mut wallet_manager)?;
            command_cpi_guard(config, token_account, owner, bulk_signers, true).await
        }
        (CommandName::DisableCpiGuard, arg_matches) => {
            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(owner_signer, &mut bulk_signers);
            }
            // Since account is required argument it will always be present
            let token_account =
                config.pubkey_or_default(arg_matches, "account", &mut wallet_manager)?;
            command_cpi_guard(config, token_account, owner, bulk_signers, false).await
        }
        (CommandName::UpdateDefaultAccountState, arg_matches) => {
            // Since account is required argument it will always be present
            let token = SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let (freeze_authority_signer, freeze_authority) =
                config.signer_or_default(arg_matches, "freeze_authority", &mut wallet_manager);
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(freeze_authority_signer, &mut bulk_signers);
            }
            let new_default_state = arg_matches.value_of("state").unwrap();
            let new_default_state = match new_default_state {
                "initialized" => AccountState::Initialized,
                "frozen" => AccountState::Frozen,
                _ => unreachable!(),
            };
            command_update_default_account_state(
                config,
                token,
                freeze_authority,
                new_default_state,
                bulk_signers,
            )
            .await
        }
        (CommandName::UpdateMetadataAddress, arg_matches) => {
            // Since account is required argument it will always be present
            let token = SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();

            let (authority_signer, authority) =
                config.signer_or_default(arg_matches, "authority", &mut wallet_manager);
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(authority_signer, &mut bulk_signers);
            }
            let metadata_address =
                SignerSource::try_get_pubkey(arg_matches, "metadata_address", &mut wallet_manager)
                    .unwrap();

            command_update_pointer_address(
                config,
                token,
                authority,
                metadata_address,
                bulk_signers,
                Pointer::Metadata,
            )
            .await
        }
        (CommandName::UpdateGroupAddress, arg_matches) => {
            // Since account is required argument it will always be present
            let token = SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();

            let (authority_signer, authority) =
                config.signer_or_default(arg_matches, "authority", &mut wallet_manager);
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(authority_signer, &mut bulk_signers);
            }
            let group_address =
                SignerSource::try_get_pubkey(arg_matches, "group_address", &mut wallet_manager)
                    .unwrap();

            command_update_pointer_address(
                config,
                token,
                authority,
                group_address,
                bulk_signers,
                Pointer::Group,
            )
            .await
        }
        (CommandName::UpdateMemberAddress, arg_matches) => {
            // Since account is required argument it will always be present
            let token = SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();

            let (authority_signer, authority) =
                config.signer_or_default(arg_matches, "authority", &mut wallet_manager);
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(authority_signer, &mut bulk_signers);
            }
            let member_address =
                SignerSource::try_get_pubkey(arg_matches, "member_address", &mut wallet_manager)
                    .unwrap();

            command_update_pointer_address(
                config,
                token,
                authority,
                member_address,
                bulk_signers,
                Pointer::GroupMember,
            )
            .await
        }
        (CommandName::WithdrawWithheldTokens, arg_matches) => {
            let (authority_signer, authority) = config.signer_or_default(
                arg_matches,
                "withdraw_withheld_authority",
                &mut wallet_manager,
            );
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(authority_signer, &mut bulk_signers);
            }
            // Since destination is required it will always be present
            let destination_token_account =
                SignerSource::try_get_pubkey(arg_matches, "account", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let include_mint = arg_matches.is_present("include_mint");
            let source_accounts =
                SignerSource::try_get_pubkeys(arg_matches, "source", &mut wallet_manager)
                    .unwrap()
                    .unwrap_or_default();
            command_withdraw_withheld_tokens(
                config,
                destination_token_account,
                source_accounts,
                authority,
                include_mint,
                bulk_signers,
            )
            .await
        }
        (CommandName::SetTransferFee, arg_matches) => {
            let token_pubkey =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                    .unwrap()
                    .unwrap();
            let transfer_fee_basis_points =
                value_t_or_exit!(arg_matches, "transfer_fee_basis_points", u16);
            let maximum_fee = *arg_matches.get_one::<Amount>("maximum_fee").unwrap();
            let (transfer_fee_authority_signer, transfer_fee_authority_pubkey) = config
                .signer_or_default(arg_matches, "transfer_fee_authority", &mut wallet_manager);
            let mint_decimals = arg_matches.get_one::<u8>(MINT_DECIMALS_ARG.name).copied();
            let bulk_signers = vec![transfer_fee_authority_signer];

            command_set_transfer_fee(
                config,
                token_pubkey,
                transfer_fee_authority_pubkey,
                transfer_fee_basis_points,
                maximum_fee,
                mint_decimals,
                bulk_signers,
            )
            .await
        }
        (CommandName::WithdrawExcessLamports, arg_matches) => {
            let (signer, authority) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(signer, &mut bulk_signers);
            }

            let source = config.pubkey_or_default(arg_matches, "from", &mut wallet_manager)?;
            let destination =
                config.pubkey_or_default(arg_matches, "recipient", &mut wallet_manager)?;

            command_withdraw_excess_lamports(config, source, destination, authority, bulk_signers)
                .await
        }
        (CommandName::UpdateConfidentialTransferSettings, arg_matches) => {
            let token_pubkey =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                    .unwrap()
                    .unwrap();

            let auto_approve = arg_matches.value_of("approve_policy").map(|b| b == "auto");

            let auditor_encryption_pubkey = if arg_matches.is_present("auditor_pubkey") {
                Some(elgamal_pubkey_or_none(arg_matches, "auditor_pubkey")?)
            } else {
                None
            };

            let (authority_signer, authority_pubkey) = config.signer_or_default(
                arg_matches,
                "confidential_transfer_authority",
                &mut wallet_manager,
            );
            let bulk_signers = vec![authority_signer];

            command_update_confidential_transfer_settings(
                config,
                token_pubkey,
                authority_pubkey,
                auto_approve,
                auditor_encryption_pubkey,
                bulk_signers,
            )
            .await
        }
        (CommandName::ConfigureConfidentialTransferAccount, arg_matches) => {
            let token =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager).unwrap();

            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);

            let account =
                SignerSource::try_get_pubkey(arg_matches, "address", &mut wallet_manager).unwrap();

            // Deriving ElGamal and AES key from signer. Custom ElGamal and AES keys will be
            // supported in the future once upgrading to clap-v3.
            //
            // NOTE:: Seed bytes are hardcoded to be empty bytes for now. They will be
            // updated once custom ElGamal and AES keys are supported.
            let elgamal_keypair = ElGamalKeypair::new_from_signer(&*owner_signer, b"").unwrap();
            let aes_key = AeKey::new_from_signer(&*owner_signer, b"").unwrap();

            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(owner_signer, &mut bulk_signers);
            }

            let maximum_credit_counter =
                if arg_matches.is_present("maximum_pending_balance_credit_counter") {
                    let maximum_credit_counter = value_t_or_exit!(
                        arg_matches.value_of("maximum_pending_balance_credit_counter"),
                        u64
                    );
                    Some(maximum_credit_counter)
                } else {
                    None
                };

            command_configure_confidential_transfer_account(
                config,
                token,
                owner,
                account,
                maximum_credit_counter,
                &elgamal_keypair,
                &aes_key,
                bulk_signers,
            )
            .await
        }
        (c @ CommandName::EnableConfidentialCredits, arg_matches)
        | (c @ CommandName::DisableConfidentialCredits, arg_matches)
        | (c @ CommandName::EnableNonConfidentialCredits, arg_matches)
        | (c @ CommandName::DisableNonConfidentialCredits, arg_matches) => {
            let token =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager).unwrap();

            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);

            let account =
                SignerSource::try_get_pubkey(arg_matches, "address", &mut wallet_manager).unwrap();

            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(owner_signer, &mut bulk_signers);
            }

            let (allow_confidential_credits, allow_non_confidential_credits) = match c {
                CommandName::EnableConfidentialCredits => (Some(true), None),
                CommandName::DisableConfidentialCredits => (Some(false), None),
                CommandName::EnableNonConfidentialCredits => (None, Some(true)),
                CommandName::DisableNonConfidentialCredits => (None, Some(false)),
                _ => (None, None),
            };

            command_enable_disable_confidential_transfers(
                config,
                token,
                owner,
                account,
                bulk_signers,
                allow_confidential_credits,
                allow_non_confidential_credits,
            )
            .await
        }
        (c @ CommandName::DepositConfidentialTokens, arg_matches)
        | (c @ CommandName::WithdrawConfidentialTokens, arg_matches) => {
            let token = SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager)
                .unwrap()
                .unwrap();
            let amount = *arg_matches.get_one::<Amount>("amount").unwrap();
            let account =
                SignerSource::try_get_pubkey(arg_matches, "address", &mut wallet_manager).unwrap();

            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);
            let mint_decimals = arg_matches.get_one::<u8>(MINT_DECIMALS_ARG.name).copied();

            let (instruction_type, elgamal_keypair, aes_key) = match c {
                CommandName::DepositConfidentialTokens => {
                    (ConfidentialInstructionType::Deposit, None, None)
                }
                CommandName::WithdrawConfidentialTokens => {
                    // Deriving ElGamal and AES key from signer. Custom ElGamal and AES keys will be
                    // supported in the future once upgrading to clap-v3.
                    //
                    // NOTE:: Seed bytes are hardcoded to be empty bytes for now. They will be
                    // updated once custom ElGamal and AES keys are supported.
                    let elgamal_keypair =
                        ElGamalKeypair::new_from_signer(&*owner_signer, b"").unwrap();
                    let aes_key = AeKey::new_from_signer(&*owner_signer, b"").unwrap();

                    (
                        ConfidentialInstructionType::Withdraw,
                        Some(elgamal_keypair),
                        Some(aes_key),
                    )
                }
                _ => panic!("Instruction not supported"),
            };

            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(owner_signer, &mut bulk_signers);
            }

            command_deposit_withdraw_confidential_tokens(
                config,
                token,
                owner,
                account,
                bulk_signers,
                amount,
                mint_decimals,
                instruction_type,
                elgamal_keypair.as_ref(),
                aes_key.as_ref(),
            )
            .await
        }
        (CommandName::ApplyPendingBalance, arg_matches) => {
            let token =
                SignerSource::try_get_pubkey(arg_matches, "token", &mut wallet_manager).unwrap();

            let (owner_signer, owner) =
                config.signer_or_default(arg_matches, "owner", &mut wallet_manager);

            let account =
                SignerSource::try_get_pubkey(arg_matches, "address", &mut wallet_manager).unwrap();

            // Deriving ElGamal and AES key from signer. Custom ElGamal and AES keys will be
            // supported in the future once upgrading to clap-v3.
            //
            // NOTE:: Seed bytes are hardcoded to be empty bytes for now. They will be
            // updated once custom ElGamal and AES keys are supported.
            let elgamal_keypair = ElGamalKeypair::new_from_signer(&*owner_signer, b"").unwrap();
            let aes_key = AeKey::new_from_signer(&*owner_signer, b"").unwrap();

            if config.multisigner_pubkeys.is_empty() {
                push_signer_with_dedup(owner_signer, &mut bulk_signers);
            }

            command_apply_pending_balance(
                config,
                token,
                owner,
                account,
                bulk_signers,
                &elgamal_keypair,
                &aes_key,
            )
            .await
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
        RpcClientResponse::Simulation(_) => {
            // Implement this once the CLI supports dry-running / simulation
            unreachable!()
        }
    }
}
