//! Token parsing

use {
    proc_macro2::{Ident, Span},
    syn::{
        parse::{Parse, ParseStream},
        token::Comma,
        LitInt, LitStr, Path, Token,
    },
};

/// Possible arguments to the `#[spl_program_error]` attribute
pub struct SplProgramErrorArgs {
    /// Whether to hash the error codes using `solana_program::hash`
    /// or to use the default error code assigned by `num_traits`.
    pub hash_error_code_start: Option<u32>,
    /// Crate to use for solana_program
    pub solana_program_crate: Path,
}

impl Parse for SplProgramErrorArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let default_solana_program_crate = Ident::new("solana_program", Span::call_site());
        let mut hash_error_code_start = None;
        let mut solana_program_crate = None;
        while !input.is_empty() {
            match SplProgramErrorArgParser::parse(input)? {
                SplProgramErrorArgParser::HashErrorCodes { value, .. } => {
                    hash_error_code_start = Some(value.base10_parse::<u32>()?);
                }
                SplProgramErrorArgParser::SolanaProgramCrate { value, .. } => {
                    solana_program_crate = value.parse()?;
                }
            }
        }
        Ok(Self {
            hash_error_code_start,
            solana_program_crate: solana_program_crate
                .unwrap_or(default_solana_program_crate)
                .into(),
        })
    }
}

/// Parser for args to the `#[spl_program_error]` attribute
/// ie. `#[spl_program_error(hash_error_code_start = 1275525928)]`
enum SplProgramErrorArgParser {
    HashErrorCodes {
        _equals_sign: Token![=],
        value: LitInt,
        _comma: Option<Comma>,
    },
    SolanaProgramCrate {
        _equals_sign: Token![=],
        value: LitStr,
        _comma: Option<Comma>,
    },
}

impl Parse for SplProgramErrorArgParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<Ident>()?;
        match ident.to_string().as_str() {
            "hash_error_code_start" => {
                let _equals_sign = input.parse::<Token![=]>()?;
                let value = input.parse::<LitInt>()?;
                let _comma: Option<Comma> = input.parse().unwrap_or(None);
                Ok(Self::HashErrorCodes {
                    _equals_sign,
                    value,
                    _comma,
                })
            }
            "solana_program_crate" => {
                let _equals_sign = input.parse::<Token![=]>()?;
                let value = input.parse::<LitStr>()?;
                let _comma: Option<Comma> = input.parse().unwrap_or(None);
                Ok(Self::SolanaProgramCrate {
                    _equals_sign,
                    value,
                    _comma,
                })
            }
            _ => {
                Err(input
                    .error("Expected argument 'hash_error_code_start' or 'solana_program_crate'"))
            }
        }
    }
}
