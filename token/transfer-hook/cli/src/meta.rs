use {
    serde::{Deserialize, Serialize},
    solana_sdk::pubkey::Pubkey,
    spl_tlv_account_resolution::{account::ExtraAccountMeta, seeds::Seed},
    std::{fmt, path::Path, str::FromStr},
    strum_macros::{EnumString, IntoStaticStr},
};

#[derive(Debug, Clone, Copy, PartialEq, EnumString, IntoStaticStr, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
enum AccountMetaRole {
    Readonly,
    Writable,
    ReadonlySigner,
    WritableSigner,
}
impl AccountMetaRole {
    fn bools(&self) -> (/* is_signer */ bool, /* is_writable */ bool) {
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

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct ExtraAccountMetaConfigFile {
    #[serde(alias = "extraMetas")]
    extra_metas: Vec<ExtraAccountMetaConfig>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct ExtraAccountMetaConfig {
    pubkey: Option<String>,
    seeds: Option<Vec<Seed>>,
    role: Option<AccountMetaRole>,
    #[serde(alias = "isSigner")]
    is_signer: Option<bool>,
    #[serde(alias = "isWritable")]
    is_writable: Option<bool>,
}
impl ExtraAccountMetaConfig {
    fn discriminator_and_address(&self) -> (u8, [u8; 32]) {
        if self.pubkey.is_some() && self.seeds.is_some() {
            panic!("Only one of `pubkey` or `seeds` must be present");
        }
        if let Some(pubkey_string) = &self.pubkey {
            (0, Pubkey::from_str(pubkey_string).unwrap().to_bytes())
        } else if let Some(seeds) = &self.seeds {
            (1, Seed::pack_into_address_config(seeds).unwrap())
        } else {
            panic!("Either `pubkey` or `seeds` must be present");
        }
    }

    fn bools(&self) -> (/* is_signer */ bool, /* is_writable */ bool) {
        if self.role.is_none() && self.is_signer.is_none() && self.is_writable.is_none() {
            panic!("Either `role` or `is_signer`/`is_writable` must be present");
        }
        if let Some(role) = self.role {
            if self.is_signer.is_some() || self.is_writable.is_some() {
                panic!("`role` and `is_signer`/`is_writable` are mutually exclusive");
            }
            role.bools()
        } else {
            if self.is_signer.is_none() || self.is_writable.is_none() {
                panic!("`is_signer` and `is_writable` must be present when `role` is not present");
            }
            (self.is_signer.unwrap(), self.is_writable.unwrap())
        }
    }
}
impl From<&ExtraAccountMetaConfig> for ExtraAccountMeta {
    fn from(config: &ExtraAccountMetaConfig) -> Self {
        let (discriminator, address_config) = config.discriminator_and_address();
        let (is_signer, is_writable) = config.bools();
        ExtraAccountMeta {
            discriminator,
            address_config,
            is_signer: is_signer.into(),
            is_writable: is_writable.into(),
        }
    }
}

pub fn parse_transfer_hook_account_arg(arg: &str) -> Result<Vec<ExtraAccountMeta>, String> {
    match arg.split(':').collect::<Vec<_>>().as_slice() {
        [address, role] => {
            let pubkey = Pubkey::from_str(address).map_err(|e| format!("{e}"))?;
            let role = AccountMetaRole::from_str(role).map_err(|e| format!("{e}"))?;
            let (is_signer, is_writable) = role.bools();
            let meta = ExtraAccountMeta::new_with_pubkey(&pubkey, is_signer, is_writable)
                .map_err(|e| format!("{e}"))?;
            Ok(vec![meta])
        }
        _ => {
            let path = Path::new(arg);
            let parse_fn = match path.extension().and_then(|s| s.to_str()) {
                Some("json") => |v: &str| {
                    serde_json::from_str::<ExtraAccountMetaConfigFile>(v)
                        .map_err(|e| format!("Unable to parse file: {e}"))
                },
                Some("yaml") | Some("yml") => |v: &str| {
                    serde_yaml::from_str::<ExtraAccountMetaConfigFile>(v)
                        .map_err(|e| format!("Unable to parse file: {e}"))
                },
                _ => panic!("Unsupported file extension: {}", path.display()),
            };
            let file = std::fs::read_to_string(path)
                .map_err(|err| format!("Unable to read file: {err}"))?;
            let parsed_config_file = parse_fn(&file)?;
            Ok(parsed_config_file
                .extra_metas
                .iter()
                .map(ExtraAccountMeta::from)
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json() {
        let config = r#"{
            "extraMetas": [
                {
                    "pubkey": "39UhVsxAmJwzPnoWhBSHsZ6nBDtdzt9D8rfDa8zGHrP6",
                    "role": "readonly-signer"
                },
                {
                    "pubkey": "6WEvW9B9jTKc3EhP1ewGEJPrxw5d8vD9eMYCf2snNYsV",
                    "isSigner": false,
                    "isWritable": false
                },
                {
                    "seeds": [
                        {
                            "literal": {
                                "bytes": [1, 2, 3, 4, 5, 6]
                            }
                        },
                        {
                            "instructionData": {
                                "index": 0,
                                "length": 8
                            }
                        },
                        {
                            "accountKey": {
                                "index": 0
                            }
                        }
                    ],
                    "role": "writable"
                },
                {
                    "seeds": [
                        {
                            "accountData": {
                                "accountIndex": 1,
                                "dataIndex": 4,
                                "length": 4
                            }
                        },
                        {
                            "accountKey": {
                                "index": 1
                            }
                        }
                    ],
                    "isSigner": false,
                    "isWritable": false
                }
            ]
        }"#;
        let parsed_config_file =
            serde_json::from_str::<ExtraAccountMetaConfigFile>(config).unwrap();
        let parsed_extra_metas: Vec<ExtraAccountMeta> = parsed_config_file
            .extra_metas
            .iter()
            .map(|config| config.into())
            .collect::<Vec<_>>();
        let expected = vec![
            ExtraAccountMeta::new_with_pubkey(
                &Pubkey::from_str("39UhVsxAmJwzPnoWhBSHsZ6nBDtdzt9D8rfDa8zGHrP6").unwrap(),
                true,
                false,
            )
            .unwrap(),
            ExtraAccountMeta::new_with_pubkey(
                &Pubkey::from_str("6WEvW9B9jTKc3EhP1ewGEJPrxw5d8vD9eMYCf2snNYsV").unwrap(),
                false,
                false,
            )
            .unwrap(),
            ExtraAccountMeta::new_with_seeds(
                &[
                    Seed::Literal {
                        bytes: vec![1, 2, 3, 4, 5, 6],
                    },
                    Seed::InstructionData {
                        index: 0,
                        length: 8,
                    },
                    Seed::AccountKey { index: 0 },
                ],
                false,
                true,
            )
            .unwrap(),
            ExtraAccountMeta::new_with_seeds(
                &[
                    Seed::AccountData {
                        account_index: 1,
                        data_index: 4,
                        length: 4,
                    },
                    Seed::AccountKey { index: 1 },
                ],
                false,
                false,
            )
            .unwrap(),
        ];
        assert_eq!(parsed_extra_metas, expected);
    }

    #[test]
    fn test_parse_yaml() {
        let config = r#"
            extra_metas:
                - pubkey: "39UhVsxAmJwzPnoWhBSHsZ6nBDtdzt9D8rfDa8zGHrP6"
                  role: "readonly-signer"
                - pubkey: "6WEvW9B9jTKc3EhP1ewGEJPrxw5d8vD9eMYCf2snNYsV"
                  is_signer: false
                  is_writable: false
                - seeds:
                    - !literal
                      bytes: [1, 2, 3, 4, 5, 6]
                    - !instruction_data
                      index: 0
                      length: 8
                    - !account_key
                      index: 0
                  role: "writable"
                - seeds:
                    - !account_data
                      account_index: 1
                      data_index: 4
                      length: 4
                    - !account_key
                      index: 1
                  is_signer: false
                  is_writable: false
        "#;
        let parsed_config_file =
            serde_yaml::from_str::<ExtraAccountMetaConfigFile>(config).unwrap();
        let parsed_extra_metas: Vec<ExtraAccountMeta> = parsed_config_file
            .extra_metas
            .iter()
            .map(|config| config.into())
            .collect::<Vec<_>>();
        let expected = vec![
            ExtraAccountMeta::new_with_pubkey(
                &Pubkey::from_str("39UhVsxAmJwzPnoWhBSHsZ6nBDtdzt9D8rfDa8zGHrP6").unwrap(),
                true,
                false,
            )
            .unwrap(),
            ExtraAccountMeta::new_with_pubkey(
                &Pubkey::from_str("6WEvW9B9jTKc3EhP1ewGEJPrxw5d8vD9eMYCf2snNYsV").unwrap(),
                false,
                false,
            )
            .unwrap(),
            ExtraAccountMeta::new_with_seeds(
                &[
                    Seed::Literal {
                        bytes: vec![1, 2, 3, 4, 5, 6],
                    },
                    Seed::InstructionData {
                        index: 0,
                        length: 8,
                    },
                    Seed::AccountKey { index: 0 },
                ],
                false,
                true,
            )
            .unwrap(),
            ExtraAccountMeta::new_with_seeds(
                &[
                    Seed::AccountData {
                        account_index: 1,
                        data_index: 4,
                        length: 4,
                    },
                    Seed::AccountKey { index: 1 },
                ],
                false,
                false,
            )
            .unwrap(),
        ];
        assert_eq!(parsed_extra_metas, expected);
    }
}
