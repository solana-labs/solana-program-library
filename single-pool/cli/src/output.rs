use {
    crate::config::Config,
    console::style,
    serde::{Deserialize, Serialize},
    serde_with::{serde_as, DisplayFromStr},
    solana_cli_output::{display::writeln_name_value, QuietDisplay, VerboseDisplay},
    solana_sdk::{pubkey::Pubkey, signature::Signature},
    spl_single_pool::{
        self, find_pool_mint_address, find_pool_mint_authority_address,
        find_pool_mpl_authority_address, find_pool_stake_address,
        find_pool_stake_authority_address,
    },
    std::fmt::{Display, Formatter, Result, Write},
};

#[derive(Debug, Serialize, Deserialize)]
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
        writeln_name_value(w, "Command:", &self.command_name)?;
        VerboseDisplay::write_str(&self.command_output, w)
    }
}

pub fn format_output<T>(config: &Config, command_name: String, command_output: T) -> String
where
    T: Serialize + Display + QuietDisplay + VerboseDisplay,
{
    config.output_format.formatted_string(&CommandOutput {
        command_name,
        command_output,
    })
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignatureOutput {
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub signature: Option<Signature>,
}

impl QuietDisplay for SignatureOutput {}
impl VerboseDisplay for SignatureOutput {}

impl Display for SignatureOutput {
    fn fmt(&self, f: &mut Formatter) -> Result {
        writeln!(f)?;

        if let Some(signature) = self.signature {
            writeln_name_value(f, "Signature:", &signature.to_string())?;
        }

        Ok(())
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StakePoolOutput {
    #[serde_as(as = "DisplayFromStr")]
    pub pool_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub vote_account_address: Pubkey,
    pub available_stake: u64,
    pub token_supply: u64,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub signature: Option<Signature>,
}

impl QuietDisplay for StakePoolOutput {}
impl VerboseDisplay for StakePoolOutput {
    fn write_str(&self, w: &mut dyn Write) -> Result {
        writeln!(w)?;
        writeln!(w, "{}", style("SPL Single-Validator Stake Pool").bold())?;
        writeln_name_value(w, "  Pool address:", &self.pool_address.to_string())?;
        writeln_name_value(
            w,
            "  Vote account address:",
            &self.vote_account_address.to_string(),
        )?;

        writeln_name_value(
            w,
            "  Pool stake address:",
            &find_pool_stake_address(&spl_single_pool::id(), &self.pool_address).to_string(),
        )?;
        writeln_name_value(
            w,
            "  Pool mint address:",
            &find_pool_mint_address(&spl_single_pool::id(), &self.pool_address).to_string(),
        )?;
        writeln_name_value(
            w,
            "  Pool stake authority address:",
            &find_pool_stake_authority_address(&spl_single_pool::id(), &self.pool_address)
                .to_string(),
        )?;
        writeln_name_value(
            w,
            "  Pool mint authority address:",
            &find_pool_mint_authority_address(&spl_single_pool::id(), &self.pool_address)
                .to_string(),
        )?;
        writeln_name_value(
            w,
            "  Pool MPL authority address:",
            &find_pool_mpl_authority_address(&spl_single_pool::id(), &self.pool_address)
                .to_string(),
        )?;

        writeln_name_value(w, "  Available stake:", &self.available_stake.to_string())?;
        writeln_name_value(w, "  Token supply:", &self.token_supply.to_string())?;

        if let Some(signature) = self.signature {
            writeln!(w)?;
            writeln_name_value(w, "Signature:", &signature.to_string())?;
        }

        Ok(())
    }
}

impl Display for StakePoolOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f)?;
        writeln!(f, "{}", style("SPL Single-Validator Stake Pool").bold())?;
        writeln_name_value(f, "  Pool address:", &self.pool_address.to_string())?;
        writeln_name_value(
            f,
            "  Vote account address:",
            &self.vote_account_address.to_string(),
        )?;
        writeln_name_value(f, "  Available stake:", &self.available_stake.to_string())?;
        writeln_name_value(f, "  Token supply:", &self.token_supply.to_string())?;

        if let Some(signature) = self.signature {
            writeln!(f)?;
            writeln_name_value(f, "Signature:", &signature.to_string())?;
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StakePoolListOutput(pub Vec<StakePoolOutput>);

impl QuietDisplay for StakePoolListOutput {}
impl VerboseDisplay for StakePoolListOutput {
    fn write_str(&self, w: &mut dyn Write) -> Result {
        let mut stake = 0;
        for svsp in &self.0 {
            VerboseDisplay::write_str(svsp, w)?;
            stake += svsp.available_stake;
        }

        writeln!(w)?;
        writeln_name_value(w, "Total stake:", &stake.to_string())?;

        Ok(())
    }
}

impl Display for StakePoolListOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let mut stake = 0;
        for svsp in &self.0 {
            svsp.fmt(f)?;
            stake += svsp.available_stake;
        }

        writeln!(f)?;
        writeln_name_value(f, "Total stake:", &stake.to_string())?;

        Ok(())
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositOutput {
    #[serde_as(as = "DisplayFromStr")]
    pub pool_address: Pubkey,
    pub token_amount: u64,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub signature: Option<Signature>,
}

impl QuietDisplay for DepositOutput {}
impl VerboseDisplay for DepositOutput {}

impl Display for DepositOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f)?;
        writeln_name_value(f, "Pool address:", &self.pool_address.to_string())?;
        writeln_name_value(f, "Token amount:", &self.token_amount.to_string())?;

        if let Some(signature) = self.signature {
            writeln!(f)?;
            writeln_name_value(f, "Signature:", &signature.to_string())?;
        }

        Ok(())
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawOutput {
    #[serde_as(as = "DisplayFromStr")]
    pub pool_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub stake_account_address: Pubkey,
    pub stake_amount: u64,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub signature: Option<Signature>,
}

impl QuietDisplay for WithdrawOutput {}
impl VerboseDisplay for WithdrawOutput {}

impl Display for WithdrawOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f)?;
        writeln_name_value(f, "Pool address:", &self.pool_address.to_string())?;
        writeln_name_value(
            f,
            "Stake account address:",
            &self.stake_account_address.to_string(),
        )?;
        writeln_name_value(f, "Stake amount:", &self.stake_amount.to_string())?;

        if let Some(signature) = self.signature {
            writeln!(f)?;
            writeln_name_value(f, "Signature:", &signature.to_string())?;
        }

        Ok(())
    }
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStakeOutput {
    #[serde_as(as = "DisplayFromStr")]
    pub pool_address: Pubkey,
    #[serde_as(as = "DisplayFromStr")]
    pub stake_account_address: Pubkey,
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub signature: Option<Signature>,
}

impl QuietDisplay for CreateStakeOutput {}
impl VerboseDisplay for CreateStakeOutput {}

impl Display for CreateStakeOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f)?;
        writeln_name_value(f, "Pool address:", &self.pool_address.to_string())?;
        writeln_name_value(
            f,
            "Stake account address:",
            &self.stake_account_address.to_string(),
        )?;

        if let Some(signature) = self.signature {
            writeln!(f)?;
            writeln_name_value(f, "Signature:", &signature.to_string())?;
        }

        Ok(())
    }
}
