use {
    serde::{Deserialize, Serialize},
    solana_sdk::pubkey::Pubkey,
    spl_tlv_account_resolution::{account::ExtraAccountMeta, seeds::Seed},
    std::{path::Path, str::FromStr},
    strum_macros::{EnumString, IntoStaticStr},
};

#[derive(Copy, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Access {
    is_signer: bool,
    is_writable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, EnumString, IntoStaticStr, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[strum(serialize_all = "camelCase")]
enum Role {
    Readonly,
    Writable,
    ReadonlySigner,
    WritableSigner,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum AddressConfig {
    Pubkey(String),
    Seeds(Vec<Seed>),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    #[serde(flatten)]
    address_config: AddressConfig,
    role: Role,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigFile {
    extra_metas: Vec<Config>,
}

impl From<&Role> for Access {
    fn from(role: &Role) -> Self {
        match role {
            Role::Readonly => Access {
                is_signer: false,
                is_writable: false,
            },
            Role::Writable => Access {
                is_signer: false,
                is_writable: true,
            },
            Role::ReadonlySigner => Access {
                is_signer: true,
                is_writable: false,
            },
            Role::WritableSigner => Access {
                is_signer: true,
                is_writable: true,
            },
        }
    }
}

impl From<&Config> for ExtraAccountMeta {
    fn from(config: &Config) -> Self {
        let Access {
            is_signer,
            is_writable,
        } = Access::from(&config.role);
        match &config.address_config {
            AddressConfig::Pubkey(pubkey_string) => ExtraAccountMeta::new_with_pubkey(
                &Pubkey::from_str(pubkey_string).unwrap(),
                is_signer,
                is_writable,
            )
            .unwrap(),
            AddressConfig::Seeds(seeds) => {
                ExtraAccountMeta::new_with_seeds(seeds, is_signer, is_writable).unwrap()
            }
        }
    }
}

type ParseFn = fn(&str) -> Result<ConfigFile, String>;

fn get_parse_function(path: &Path) -> Result<ParseFn, String> {
    match path.extension().and_then(|s| s.to_str()) {
        Some("json") => Ok(|v: &str| {
            serde_json::from_str::<ConfigFile>(v).map_err(|e| format!("Unable to parse file: {e}"))
        }),
        Some("yaml") | Some("yml") => Ok(|v: &str| {
            serde_yaml::from_str::<ConfigFile>(v).map_err(|e| format!("Unable to parse file: {e}"))
        }),
        _ => Err(format!(
            "Unsupported file extension: {}. Only JSON and YAML files are supported",
            path.display()
        )),
    }
}

fn parse_config_file_arg(path_str: &str) -> Result<Vec<ExtraAccountMeta>, String> {
    let path = Path::new(path_str);
    let parse_fn = get_parse_function(path)?;
    let file =
        std::fs::read_to_string(path).map_err(|err| format!("Unable to read file: {err}"))?;
    let parsed_config_file = parse_fn(&file)?;
    Ok(parsed_config_file
        .extra_metas
        .iter()
        .map(ExtraAccountMeta::from)
        .collect())
}

fn parse_pubkey_role_arg(pubkey_string: &str, role: &str) -> Result<Vec<ExtraAccountMeta>, String> {
    let pubkey = Pubkey::from_str(pubkey_string).map_err(|e| format!("{e}"))?;
    let role = &Role::from_str(role).map_err(|e| format!("{e}"))?;
    let Access {
        is_signer,
        is_writable,
    } = role.into();
    ExtraAccountMeta::new_with_pubkey(&pubkey, is_signer, is_writable)
        .map(|meta| vec![meta])
        .map_err(|e| format!("{e}"))
}

pub fn parse_transfer_hook_account_arg(arg: &str) -> Result<Vec<ExtraAccountMeta>, String> {
    match arg.split(':').collect::<Vec<_>>().as_slice() {
        [pubkey_str, role] => parse_pubkey_role_arg(pubkey_str, role),
        _ => parse_config_file_arg(arg),
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
                    "role": "readonlySigner"
                },
                {
                    "pubkey": "6WEvW9B9jTKc3EhP1ewGEJPrxw5d8vD9eMYCf2snNYsV",
                    "role": "readonly"
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
                    "role": "readonly"
                }
            ]
        }"#;
        let parsed_config_file = serde_json::from_str::<ConfigFile>(config).unwrap();
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
            extraMetas:
                - pubkey: "39UhVsxAmJwzPnoWhBSHsZ6nBDtdzt9D8rfDa8zGHrP6"
                  role: "readonlySigner"
                - pubkey: "6WEvW9B9jTKc3EhP1ewGEJPrxw5d8vD9eMYCf2snNYsV"
                  role: "readonly"
                - seeds:
                    - literal:
                        bytes: [1, 2, 3, 4, 5, 6]
                    - instructionData:
                        index: 0
                        length: 8
                    - accountKey:
                        index: 0
                  role: "writable"
                - seeds:
                    - accountData:
                        accountIndex: 1
                        dataIndex: 4
                        length: 4
                    - accountKey:
                        index: 1
                  role: "readonly"
        "#;
        let parsed_config_file = serde_yaml::from_str::<ConfigFile>(config).unwrap();
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
