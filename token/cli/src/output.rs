use crate::{config::Config, sort::UnsupportedAccount};
use console::{style, Emoji};
use serde::{Deserialize, Serialize, Serializer};
use solana_account_decoder::{
    parse_token::{UiAccountState, UiMint, UiMultisig, UiTokenAccount, UiTokenAmount},
    parse_token_extension::{
        UiDefaultAccountState, UiExtension, UiInterestBearingConfig, UiMemoTransfer,
        UiMintCloseAuthority, UiTransferFeeAmount, UiTransferFeeConfig,
    },
};
use solana_cli_output::{display::writeln_name_value, OutputFormat, QuietDisplay, VerboseDisplay};
use std::fmt::{self, Display};

pub(crate) trait Output: Serialize + fmt::Display + QuietDisplay + VerboseDisplay {}

static WARNING: Emoji = Emoji("⚠️", "!");

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommandOutput<T>
where
    T: Serialize + Display + QuietDisplay + VerboseDisplay,
{
    pub(crate) command_name: String,
    pub(crate) command_output: T,
}

impl<T> Display for CommandOutput<T>
where
    T: Serialize + Display + QuietDisplay + VerboseDisplay,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.command_output, f)
    }
}

impl<T> QuietDisplay for CommandOutput<T>
where
    T: Serialize + Display + QuietDisplay + VerboseDisplay,
{
    fn write_str(&self, w: &mut dyn std::fmt::Write) -> std::fmt::Result {
        QuietDisplay::write_str(&self.command_output, w)
    }
}

impl<T> VerboseDisplay for CommandOutput<T>
where
    T: Serialize + Display + QuietDisplay + VerboseDisplay,
{
    fn write_str(&self, w: &mut dyn std::fmt::Write) -> std::fmt::Result {
        writeln_name_value(w, "Command: ", &self.command_name)?;
        VerboseDisplay::write_str(&self.command_output, w)
    }
}

pub(crate) fn println_display(config: &Config, message: String) {
    match config.output_format {
        OutputFormat::Display | OutputFormat::DisplayVerbose => {
            println!("{}", message);
        }
        _ => {}
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliCreateToken<T>
where
    T: Serialize + Display + QuietDisplay + VerboseDisplay,
{
    pub(crate) address: String,
    pub(crate) decimals: u8,
    pub(crate) transaction_data: T,
}

impl<T> Display for CliCreateToken<T>
where
    T: Serialize + Display + QuietDisplay + VerboseDisplay,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f)?;
        writeln_name_value(f, "Address: ", &self.address)?;
        writeln_name_value(f, "Decimals: ", &format!("{}", self.decimals))?;
        Display::fmt(&self.transaction_data, f)
    }
}
impl<T> QuietDisplay for CliCreateToken<T>
where
    T: Serialize + Display + QuietDisplay + VerboseDisplay,
{
    fn write_str(&self, w: &mut dyn std::fmt::Write) -> std::fmt::Result {
        writeln!(w)?;
        writeln_name_value(w, "Address: ", &self.address)?;
        writeln_name_value(w, "Decimals: ", &format!("{}", self.decimals))?;
        QuietDisplay::write_str(&self.transaction_data, w)
    }
}
impl<T> VerboseDisplay for CliCreateToken<T>
where
    T: Serialize + Display + QuietDisplay + VerboseDisplay,
{
    fn write_str(&self, w: &mut dyn std::fmt::Write) -> std::fmt::Result {
        writeln!(w)?;
        writeln_name_value(w, "Address: ", &self.address)?;
        writeln_name_value(w, "Decimals: ", &format!("{}", self.decimals))?;
        VerboseDisplay::write_str(&self.transaction_data, w)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliTokenAmount {
    #[serde(flatten)]
    pub(crate) amount: UiTokenAmount,
}

impl QuietDisplay for CliTokenAmount {}
impl VerboseDisplay for CliTokenAmount {
    fn write_str(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        writeln!(w, "ui amount: {}", self.amount.real_number_string_trimmed())?;
        writeln!(w, "decimals: {}", self.amount.decimals)?;
        writeln!(w, "amount: {}", self.amount.amount)
    }
}

impl fmt::Display for CliTokenAmount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "{}", self.amount.real_number_string_trimmed())
    }
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliWalletAddress {
    pub(crate) wallet_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) associated_token_address: Option<String>,
}

impl QuietDisplay for CliWalletAddress {}
impl VerboseDisplay for CliWalletAddress {}

impl fmt::Display for CliWalletAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f, "Wallet address: {}", self.wallet_address)?;
        if let Some(associated_token_address) = &self.associated_token_address {
            writeln!(f, "Associated token address: {}", associated_token_address)?;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliMultisig {
    pub(crate) address: String,
    pub(crate) program_id: String,
    #[serde(flatten)]
    pub(crate) multisig: UiMultisig,
}

impl QuietDisplay for CliMultisig {}
impl VerboseDisplay for CliMultisig {}

impl fmt::Display for CliMultisig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let m = self.multisig.num_required_signers;
        let n = self.multisig.num_valid_signers;

        writeln!(f)?;
        writeln!(f, "{}", style("SPL Token Multisig").bold())?;
        writeln_name_value(f, "  Address:", &self.address)?;
        writeln_name_value(f, "  Program:", &self.program_id)?;
        writeln_name_value(f, "  M/N:", &format!("{}/{}", m, n))?;
        writeln!(f, "  {}", style("Signers:").bold())?;
        let width = if n >= 9 { 4 } else { 3 };
        for i in 0..n as usize {
            let title = format!("  {1:>0$}:", width, i + 1);
            let pubkey = &self.multisig.signers[i];
            writeln_name_value(f, &title, pubkey)?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliTokenAccount {
    pub(crate) address: String,
    pub(crate) program_id: String,
    pub(crate) is_associated: bool,
    #[serde(flatten)]
    pub(crate) account: UiTokenAccount,
}

impl QuietDisplay for CliTokenAccount {}
impl VerboseDisplay for CliTokenAccount {}

impl fmt::Display for CliTokenAccount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f)?;
        writeln!(f, "{}", style("SPL Token Account").bold())?;
        if self.is_associated {
            writeln_name_value(f, "  Address:", &self.address)?;
        } else {
            writeln_name_value(f, "  Address:", &format!("{}  (Aux*)", self.address))?;
        }
        writeln_name_value(f, "  Program:", &self.program_id)?;
        writeln_name_value(
            f,
            "  Balance:",
            &self.account.token_amount.real_number_string_trimmed(),
        )?;
        writeln_name_value(
            f,
            "  Decimals:",
            self.account.token_amount.decimals.to_string().as_ref(),
        )?;
        let mint = format!(
            "{}{}",
            self.account.mint,
            if self.account.is_native {
                " (native)"
            } else {
                ""
            }
        );
        writeln_name_value(f, "  Mint:", &mint)?;
        writeln_name_value(f, "  Owner:", &self.account.owner)?;
        writeln_name_value(f, "  State:", &format!("{:?}", self.account.state))?;
        if let Some(delegate) = &self.account.delegate {
            writeln!(f, "  {}", style("Delegation:").bold())?;
            writeln_name_value(f, "    Delegate:", delegate)?;
            let allowance = self.account.delegated_amount.as_ref().unwrap();
            writeln_name_value(f, "    Allowance:", &allowance.real_number_string_trimmed())?;
        } else {
            writeln_name_value(f, "  Delegation:", "")?;
        }
        writeln_name_value(
            f,
            "  Close authority:",
            self.account
                .close_authority
                .as_ref()
                .unwrap_or(&String::new()),
        )?;

        if !self.account.extensions.is_empty() {
            writeln!(f, "{}", style("Extensions:").bold())?;
            for extension in &self.account.extensions {
                display_ui_extension(f, 0, extension)?;
            }
        }

        if !self.is_associated {
            writeln!(f)?;
            writeln!(f, "* Please run `spl-token gc` to clean up Aux accounts")?;
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliMint {
    pub(crate) address: String,
    pub(crate) program_id: String,
    #[serde(skip_serializing)]
    pub(crate) epoch: u64,
    #[serde(flatten)]
    pub(crate) mint: UiMint,
}

impl QuietDisplay for CliMint {}
impl VerboseDisplay for CliMint {}

impl fmt::Display for CliMint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f)?;
        writeln!(f, "{}", style("SPL Token Mint").bold())?;

        writeln_name_value(f, "  Address:", &self.address)?;
        writeln_name_value(f, "  Program:", &self.program_id)?;
        writeln_name_value(f, "  Supply:", &self.mint.supply)?;
        writeln_name_value(f, "  Decimals:", &self.mint.decimals.to_string())?;
        writeln_name_value(
            f,
            "  Mint authority:",
            self.mint.mint_authority.as_ref().unwrap_or(&String::new()),
        )?;
        writeln_name_value(
            f,
            "  Freeze authority:",
            self.mint
                .freeze_authority
                .as_ref()
                .unwrap_or(&String::new()),
        )?;

        if !self.mint.extensions.is_empty() {
            writeln!(f, "{}", style("Extensions").bold())?;
            for extension in &self.mint.extensions {
                display_ui_extension(f, self.epoch, extension)?;
            }
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliTokenAccounts {
    #[serde(serialize_with = "flattened")]
    pub(crate) accounts: Vec<Vec<CliTokenAccount>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) unsupported_accounts: Vec<UnsupportedAccount>,
    #[serde(skip_serializing)]
    pub(crate) max_len_balance: usize,
    #[serde(skip_serializing)]
    pub(crate) aux_len: usize,
    #[serde(skip_serializing)]
    pub(crate) explicit_token: bool,
}

impl QuietDisplay for CliTokenAccounts {}
impl VerboseDisplay for CliTokenAccounts {
    fn write_str(&self, w: &mut dyn fmt::Write) -> fmt::Result {
        let mut gc_alert = false;

        let mut delegate_padding = 9;
        let mut close_authority_padding = 15;
        for accounts_list in self.accounts.iter() {
            for account in accounts_list {
                if account.account.delegated_amount.is_some() {
                    delegate_padding = delegate_padding.max(
                        account
                            .account
                            .delegated_amount
                            .as_ref()
                            .unwrap()
                            .amount
                            .len(),
                    );
                }

                if account.account.close_authority.is_some() {
                    close_authority_padding = 44;
                }
            }
        }

        let header = if self.explicit_token {
            format!(
                "{:<44}  {:<44}  {:<5$}  {:<6$}  {:<7$}",
                "Program",
                "Account",
                "Delegated",
                "Close Authority",
                "Balance",
                delegate_padding,
                close_authority_padding,
                self.max_len_balance
            )
        } else {
            format!(
                "{:<44}  {:<44}  {:<44}  {:<6$}  {:<7$}  {:<8$}",
                "Program",
                "Token",
                "Account",
                "Delegated",
                "Close Authority",
                "Balance",
                delegate_padding,
                close_authority_padding,
                self.max_len_balance
            )
        };
        writeln!(w, "{}", header)?;
        writeln!(w, "{}", "-".repeat(header.len() + self.aux_len))?;

        for accounts_list in self.accounts.iter() {
            let mut aux_counter = 1;
            for account in accounts_list {
                let maybe_aux = if !account.is_associated {
                    gc_alert = true;
                    let message = format!("  (Aux-{}*)", aux_counter);
                    aux_counter += 1;
                    message
                } else {
                    "".to_string()
                };

                let maybe_frozen = if let UiAccountState::Frozen = account.account.state {
                    format!(" {}  Frozen", WARNING)
                } else {
                    "".to_string()
                };

                let maybe_delegated = account
                    .account
                    .delegated_amount
                    .clone()
                    .map(|d| d.amount)
                    .unwrap_or_else(|| "".to_string());

                let maybe_close_authority = account
                    .account
                    .close_authority
                    .clone()
                    .unwrap_or_else(|| "".to_string());

                if self.explicit_token {
                    writeln!(
                        w,
                        "{:<44}  {:<44}  {:<7$}  {:<8$}  {:<9$}{:<10$}{}",
                        account.program_id,
                        account.address,
                        maybe_delegated,
                        maybe_close_authority,
                        account.account.token_amount.real_number_string_trimmed(),
                        maybe_aux,
                        maybe_frozen,
                        delegate_padding,
                        close_authority_padding,
                        self.max_len_balance,
                        self.aux_len,
                    )?;
                } else {
                    writeln!(
                        w,
                        "{:<44}  {:<44}  {:<44}  {:<8$}  {:<9$}  {:<10$}{:<11$}{}",
                        account.program_id,
                        account.account.mint,
                        account.address,
                        maybe_delegated,
                        maybe_close_authority,
                        account.account.token_amount.real_number_string_trimmed(),
                        maybe_aux,
                        maybe_frozen,
                        delegate_padding,
                        close_authority_padding,
                        self.max_len_balance,
                        self.aux_len,
                    )?;
                }
            }
        }
        for unsupported_account in &self.unsupported_accounts {
            writeln!(
                w,
                "{:<44}  {}",
                unsupported_account.address, unsupported_account.err
            )?;
        }
        if gc_alert {
            writeln!(w)?;
            writeln!(w, "* Please run `spl-token gc` to clean up Aux accounts")?;
        }
        Ok(())
    }
}

impl fmt::Display for CliTokenAccounts {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut gc_alert = false;
        let header = if self.explicit_token {
            format!("{:<1$}", "Balance", self.max_len_balance)
        } else {
            format!("{:<44}  {:<2$}", "Token", "Balance", self.max_len_balance)
        };
        writeln!(f, "{}", header)?;
        writeln!(f, "{}", "-".repeat(header.len() + self.aux_len))?;

        for accounts_list in self.accounts.iter() {
            let mut aux_counter = 1;
            for account in accounts_list {
                let maybe_aux = if !account.is_associated {
                    gc_alert = true;
                    let message = format!("  (Aux-{}*)", aux_counter);
                    aux_counter += 1;
                    message
                } else {
                    "".to_string()
                };
                let maybe_frozen = if let UiAccountState::Frozen = account.account.state {
                    format!(" {}  Frozen", WARNING)
                } else {
                    "".to_string()
                };
                if self.explicit_token {
                    writeln!(
                        f,
                        "{:<3$}{:<4$}{}",
                        account.account.token_amount.real_number_string_trimmed(),
                        maybe_aux,
                        maybe_frozen,
                        self.max_len_balance,
                        self.aux_len,
                    )?;
                } else {
                    writeln!(
                        f,
                        "{:<44}  {:<4$}{:<5$}{}",
                        account.account.mint,
                        account.account.token_amount.real_number_string_trimmed(),
                        maybe_aux,
                        maybe_frozen,
                        self.max_len_balance,
                        self.aux_len,
                    )?;
                }
            }
        }
        for unsupported_account in &self.unsupported_accounts {
            writeln!(
                f,
                "{:<44}  {}",
                unsupported_account.address, unsupported_account.err
            )?;
        }
        if gc_alert {
            writeln!(f)?;
            writeln!(f, "* Please run `spl-token gc` to clean up Aux accounts")?;
        }
        Ok(())
    }
}

fn display_ui_extension(
    f: &mut fmt::Formatter,
    epoch: u64,
    ui_extension: &UiExtension,
) -> fmt::Result {
    match ui_extension {
        UiExtension::TransferFeeConfig(UiTransferFeeConfig {
            transfer_fee_config_authority,
            withdraw_withheld_authority,
            withheld_amount,
            older_transfer_fee,
            newer_transfer_fee,
        }) => {
            writeln!(f, "  {}", style("Transfer fees:").bold())?;

            if newer_transfer_fee.epoch >= epoch {
                writeln!(
                    f,
                    "    {} {}bps",
                    style("Current fee:").bold(),
                    newer_transfer_fee.transfer_fee_basis_points
                )?;
                writeln_name_value(
                    f,
                    "    Current maximum:",
                    &newer_transfer_fee.maximum_fee.to_string(),
                )?;
            } else {
                writeln!(
                    f,
                    "    {} {}bps",
                    style("Current fee:").bold(),
                    older_transfer_fee.transfer_fee_basis_points
                )?;
                writeln_name_value(
                    f,
                    "    Current maximum:",
                    &older_transfer_fee.maximum_fee.to_string(),
                )?;
                writeln!(
                    f,
                    "    {} {}bps",
                    style("Upcoming fee:").bold(),
                    newer_transfer_fee.transfer_fee_basis_points
                )?;
                writeln_name_value(
                    f,
                    "    Upcoming maximum:",
                    &newer_transfer_fee.maximum_fee.to_string(),
                )?;
                writeln!(
                    f,
                    "    {} Epoch {} ({} epochs)",
                    style("Switchover at:").bold(),
                    newer_transfer_fee.epoch,
                    newer_transfer_fee.epoch - epoch
                )?;
            }

            writeln_name_value(
                f,
                "    Config authority:",
                transfer_fee_config_authority
                    .as_ref()
                    .unwrap_or(&String::new()),
            )?;
            writeln_name_value(
                f,
                "    Withdrawal authority:",
                withdraw_withheld_authority
                    .as_ref()
                    .unwrap_or(&String::new()),
            )?;
            writeln_name_value(f, "    Withheld fees:", &withheld_amount.to_string())
        }
        UiExtension::TransferFeeAmount(UiTransferFeeAmount { withheld_amount }) => {
            writeln_name_value(f, "  Transfer fees withheld:", &withheld_amount.to_string())
        }
        UiExtension::MintCloseAuthority(UiMintCloseAuthority { close_authority }) => {
            writeln_name_value(
                f,
                "  Close authority:",
                close_authority.as_ref().unwrap_or(&String::new()),
            )
        }
        UiExtension::ConfidentialTransferMint(_) => unimplemented!(),
        UiExtension::ConfidentialTransferAccount(_) => unimplemented!(),
        UiExtension::DefaultAccountState(UiDefaultAccountState { account_state }) => {
            writeln_name_value(f, "  Default state:", &format!("{:?}", account_state))
        }
        UiExtension::ImmutableOwner => writeln!(f, "  {}", style("Immutable owner").bold()),
        UiExtension::MemoTransfer(UiMemoTransfer {
            require_incoming_transfer_memos,
        }) => writeln_name_value(
            f,
            "  Transfer memo:",
            if *require_incoming_transfer_memos {
                "Required"
            } else {
                "Not required"
            },
        ),
        UiExtension::NonTransferable => writeln!(f, "  {}", style("Non-transferable").bold()),
        UiExtension::InterestBearingConfig(UiInterestBearingConfig {
            rate_authority,
            pre_update_average_rate,
            current_rate,
            ..
        }) => {
            writeln!(f, "  {}", style("Interest-bearing:").bold())?;
            writeln!(
                f,
                "    {} {}bps",
                style("Current rate:").bold(),
                current_rate
            )?;
            writeln!(
                f,
                "    {} {}bps",
                style("Average rate:").bold(),
                pre_update_average_rate
            )?;
            writeln_name_value(
                f,
                "    Rate authority:",
                rate_authority.as_ref().unwrap_or(&String::new()),
            )
        }
        // ExtensionType::Uninitialized is a hack to ensure a mint/account is never the same length as a multisig
        UiExtension::Uninitialized => Ok(()),
        UiExtension::UnparseableExtension => writeln_name_value(
            f,
            "    Unparseable extension:",
            "Consider upgrading to a newer version of spl-token",
        ),
        // XXX HANA this is needed temporarily to make monorepo ci happy while the new cases are added
        #[allow(unreachable_patterns)]
        _ => unimplemented!(),
    }
}

fn flattened<S: Serializer>(
    vec: &[Vec<CliTokenAccount>],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let flattened: Vec<_> = vec.iter().flatten().collect();
    flattened.serialize(serializer)
}
