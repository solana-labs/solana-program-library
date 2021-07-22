use crate::config::Config;
use serde::{Deserialize, Serialize};
use solana_account_decoder::parse_token::{UiTokenAccount, UiTokenAmount};
use solana_cli_output::{display::writeln_name_value, OutputFormat, QuietDisplay, VerboseDisplay};
use std::fmt;

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
    pub(crate) m: u8,
    pub(crate) n: u8,
    pub(crate) signers: Vec<String>,
}

impl QuietDisplay for CliMultisig {}
impl VerboseDisplay for CliMultisig {}

impl fmt::Display for CliMultisig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f)?;
        writeln_name_value(f, "Address:", &self.address)?;
        writeln_name_value(f, "M/N:", &format!("{}/{}", self.m, self.n))?;
        writeln_name_value(f, "Signers:", " ")?;
        let width = if self.n >= 9 { 4 } else { 3 };
        for i in 0..self.n as usize {
            let title = format!("{1:>0$}:", width, i + 1);
            let pubkey = &self.signers[i];
            writeln_name_value(f, &title, pubkey)?;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CliTokenAccount {
    pub(crate) address: String,
    pub(crate) is_associated: bool,
    #[serde(flatten)]
    pub(crate) account: UiTokenAccount,
}

impl QuietDisplay for CliTokenAccount {}
impl VerboseDisplay for CliTokenAccount {}

impl fmt::Display for CliTokenAccount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f)?;
        if self.is_associated {
            writeln_name_value(f, "Address:", &self.address)?;
        } else {
            writeln_name_value(f, "Address:", &format!("{}  (Aux*)", self.address))?;
        }
        writeln_name_value(
            f,
            "Balance:",
            &self.account.token_amount.real_number_string_trimmed(),
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
        writeln_name_value(f, "Mint:", &mint)?;
        writeln_name_value(f, "Owner:", &self.account.owner)?;
        writeln_name_value(f, "State:", &format!("{:?}", self.account.state))?;
        if let Some(delegate) = &self.account.delegate {
            writeln!(f, "Delegation:")?;
            writeln_name_value(f, "  Delegate:", delegate)?;
            let allowance = self.account.delegated_amount.as_ref().unwrap();
            writeln_name_value(f, "  Allowance:", &allowance.real_number_string_trimmed())?;
        } else {
            writeln_name_value(f, "Delegation:", "")?;
        }
        writeln_name_value(
            f,
            "Close authority:",
            self.account
                .close_authority
                .as_ref()
                .unwrap_or(&String::new()),
        )?;
        if !self.is_associated {
            writeln!(f)?;
            writeln!(f, "* Please run `spl-token gc` to clean up Aux accounts")?;
        }
        Ok(())
    }
}
