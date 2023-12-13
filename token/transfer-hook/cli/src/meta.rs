use {
    solana_sdk::{instruction::AccountMeta, pubkey::Pubkey},
    std::{fmt, str::FromStr},
    strum_macros::{EnumString, IntoStaticStr},
};

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

pub fn parse_transfer_hook_account(arg: &str) -> Result<AccountMeta, String> {
    match arg.split(':').collect::<Vec<_>>().as_slice() {
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
