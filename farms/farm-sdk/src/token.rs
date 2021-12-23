//! Token

use {
    crate::{pack::*, string::ArrayString64, traits::*},
    arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs},
    num_enum::TryFromPrimitive,
    serde::{Deserialize, Serialize},
    serde_json::to_string,
    solana_program::{program_error::ProgramError, pubkey::Pubkey},
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
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum TokenSelector {
    TokenA,
    TokenB,
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
}

impl Named for Token {
    fn name(&self) -> ArrayString64 {
        self.name
    }
}

impl Token {
    pub const LEN: usize = 171;

    pub fn get_size(&self) -> usize {
        Token::LEN
    }

    pub fn pack(&self, output: &mut [u8]) -> Result<usize, ProgramError> {
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
        ) = mut_array_refs![output, 64, 64, 1, 5, 2, 1, 2, 32];
        pack_array_string64(&self.name, name_out);
        pack_array_string64(&self.description, description_out);
        token_type_out[0] = self.token_type as u8;
        pack_option_u32(self.refdb_index, refdb_index_out);
        *refdb_counter_out = self.refdb_counter.to_le_bytes();
        decimals_out[0] = self.decimals;
        *chain_id_out = self.chain_id.to_le_bytes();
        mint_out.copy_from_slice(self.mint.as_ref());

        Ok(Token::LEN)
    }

    pub fn to_vec(&self) -> Result<Vec<u8>, ProgramError> {
        let mut output: [u8; Token::LEN] = [0; Token::LEN];
        if let Ok(len) = self.pack(&mut output[..]) {
            Ok(output[..len].to_vec())
        } else {
            Err(ProgramError::InvalidAccountData)
        }
    }

    pub fn unpack(input: &[u8]) -> Result<Token, ProgramError> {
        check_data_len(input, Token::LEN)?;

        let input = array_ref![input, 0, Token::LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (name, description, token_type, refdb_index, refdb_counter, decimals, chain_id, mint) =
            array_refs![input, 64, 64, 1, 5, 2, 1, 2, 32];

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
        }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", to_string(&self).unwrap())
    }
}
