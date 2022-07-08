//! Token

use {
    crate::{pack::*, string::ArrayString64, traits::*},
    arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs},
    num_enum::TryFromPrimitive,
    serde::{Deserialize, Serialize},
    serde_json::{to_string, Value},
    solana_program::{program_error::ProgramError, pubkey::Pubkey},
    std::collections::HashMap,
};

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum TokenType {
    NativeSol,
    WrappedSol,
    WrappedSollet,
    WrappedWarmhole,
    SplToken,
    LpToken,
    VtToken,
    FundToken,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum TokenSelector {
    TokenA,
    TokenB,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum OracleType {
    Pyth,
    Chainlink,
    Unsupported,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub struct OraclePrice {
    pub price: u64,
    pub exponent: i32,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub struct Token {
    #[serde(
        serialize_with = "as64_serialize",
        deserialize_with = "as64_deserialize"
    )]
    pub name: ArrayString64,
    #[serde(
        serialize_with = "as64_serialize",
        deserialize_with = "as64_deserialize"
    )]
    pub description: ArrayString64,
    pub token_type: TokenType,
    pub refdb_index: Option<u32>,
    pub refdb_counter: u16,
    pub decimals: u8,
    pub chain_id: u16,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub mint: Pubkey,
    pub oracle_type: OracleType,
    #[serde(
        deserialize_with = "optional_pubkey_deserialize",
        serialize_with = "optional_pubkey_serialize"
    )]
    pub oracle_account: Option<Pubkey>,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub description_account: Pubkey,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GitToken {
    #[serde(rename = "chainId")]
    pub chain_id: i32,
    #[serde(
        deserialize_with = "pubkey_deserialize",
        serialize_with = "pubkey_serialize"
    )]
    pub address: Pubkey,
    pub symbol: String,
    pub name: String,
    pub decimals: i32,
    #[serde(rename = "logoURI", default)]
    pub logo_uri: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl Named for Token {
    fn name(&self) -> ArrayString64 {
        self.name
    }
}

impl Token {
    pub const LEN: usize = 237;
}

impl Packed for Token {
    fn get_size(&self) -> usize {
        Token::LEN
    }

    fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
        check_data_len(output, Token::LEN)?;

        let output = array_mut_ref![output, 0, Token::LEN];

        let (
            name_out,
            description_out,
            token_type_out,
            refdb_index_out,
            refdb_counter_out,
            decimals_out,
            chain_id_out,
            mint_out,
            oracle_type_out,
            oracle_account_out,
            description_account_out,
        ) = mut_array_refs![output, 64, 64, 1, 5, 2, 1, 2, 32, 1, 33, 32];
        pack_array_string64(&self.name, name_out);
        pack_array_string64(&self.description, description_out);
        token_type_out[0] = self.token_type as u8;
        pack_option_u32(self.refdb_index, refdb_index_out);
        *refdb_counter_out = self.refdb_counter.to_le_bytes();
        decimals_out[0] = self.decimals;
        *chain_id_out = self.chain_id.to_le_bytes();
        mint_out.copy_from_slice(self.mint.as_ref());
        oracle_type_out[0] = self.oracle_type as u8;
        pack_option_key(&self.oracle_account, oracle_account_out);
        description_account_out.copy_from_slice(self.description_account.as_ref());

        Ok(Token::LEN)
    }

    fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; Token::LEN] = [0; Token::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    fn unpack(input: &[u8]) -> Result<Token, ProgramError> {
        check_data_len(input, Token::LEN)?;

        let input = array_ref![input, 0, Token::LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            name,
            description,
            token_type,
            refdb_index,
            refdb_counter,
            decimals,
            chain_id,
            mint,
            oracle_type,
            oracle_account,
            description_account,
        ) = array_refs![input, 64, 64, 1, 5, 2, 1, 2, 32, 1, 33, 32];

        Ok(Self {
            name: unpack_array_string64(name)?,
            description: unpack_array_string64(description)?,
            token_type: TokenType::try_from_primitive(token_type[0])
                .or(Err(ProgramError::InvalidAccountData))?,
            refdb_index: unpack_option_u32(refdb_index)?,
            refdb_counter: u16::from_le_bytes(*refdb_counter),
            decimals: decimals[0],
            chain_id: u16::from_le_bytes(*chain_id),
            mint: Pubkey::new_from_array(*mint),
            oracle_type: OracleType::try_from_primitive(oracle_type[0])
                .or(Err(ProgramError::InvalidAccountData))?,
            oracle_account: unpack_option_key(oracle_account)?,
            description_account: Pubkey::new_from_array(*description_account),
        })
    }
}

impl std::fmt::Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            TokenType::NativeSol => write!(f, "NativeSol"),
            TokenType::WrappedSol => write!(f, "WrappedSol"),
            TokenType::WrappedSollet => write!(f, "WrappedSollet"),
            TokenType::WrappedWarmhole => write!(f, "WrappedWarmhole"),
            TokenType::SplToken => write!(f, "SplToken"),
            TokenType::LpToken => write!(f, "LpToken"),
            TokenType::VtToken => write!(f, "VtToken"),
            TokenType::FundToken => write!(f, "FundToken"),
        }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}

impl std::fmt::Display for TokenSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            TokenSelector::TokenA => write!(f, "TokenA"),
            TokenSelector::TokenB => write!(f, "TokenB"),
        }
    }
}

impl std::str::FromStr for TokenSelector {
    type Err = ProgramError;

    fn from_str(s: &str) -> Result<Self, ProgramError> {
        match s.to_lowercase().as_str() {
            "tokena" => Ok(TokenSelector::TokenA),
            "tokenb" => Ok(TokenSelector::TokenB),
            _ => Err(ProgramError::InvalidArgument),
        }
    }
}

impl std::fmt::Display for OracleType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            OracleType::Pyth => write!(f, "Pyth"),
            OracleType::Chainlink => write!(f, "Chainlink"),
            OracleType::Unsupported => write!(f, "Unsupported"),
        }
    }
}

impl std::str::FromStr for OracleType {
    type Err = ProgramError;

    fn from_str(s: &str) -> Result<Self, ProgramError> {
        match s.to_lowercase().as_str() {
            "pyth" => Ok(OracleType::Pyth),
            "chainlink" => Ok(OracleType::Chainlink),
            "unsupported" => Ok(OracleType::Unsupported),
            _ => Err(ProgramError::InvalidArgument),
        }
    }
}
