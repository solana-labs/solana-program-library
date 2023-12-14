use {
    solana_sdk::pubkey::Pubkey,
    spl_tlv_account_resolution::account::ExtraAccountMeta,
    std::{fmt, str::FromStr},
    strum_macros::{EnumString, IntoStaticStr},
};

#[derive(Debug, Clone, Copy, PartialEq, EnumString, IntoStaticStr)]
#[strum(serialize_all = "kebab-case")]
enum AccountMetaRole {
    Readonly,
    Writable,
    ReadonlySigner,
    WritableSigner,
}
impl AccountMetaRole {
    fn bools(&self) -> (bool, bool) {
        match self {
            AccountMetaRole::Readonly => (false, false),
            AccountMetaRole::Writable => (false, true),
            AccountMetaRole::ReadonlySigner => (true, false),
            AccountMetaRole::WritableSigner => (true, true),
        }
    }
}
impl fmt::Display for AccountMetaRole {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn parse_transfer_hook_account_arg(arg: &str) -> Result<ExtraAccountMeta, String> {
    match arg.split(':').collect::<Vec<_>>().as_slice() {
        [address, role] => {
            let pubkey = Pubkey::from_str(address).map_err(|e| format!("{e}"))?;
            let role = AccountMetaRole::from_str(role).map_err(|e| format!("{e}"))?;
            let (is_signer, is_writable) = role.bools();
            ExtraAccountMeta::new_with_pubkey(&pubkey, is_signer, is_writable)
                .map_err(|e| format!("{e}"))
        }
        _ => Err("Transfer hook account must be present as <ADDRESS>:<ROLE>".to_string()),
    }
}
