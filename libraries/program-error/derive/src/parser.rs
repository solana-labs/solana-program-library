use {
    proc_macro2::Ident,
    syn::{
        parse::{Parse, ParseStream},
        token::Comma,
        LitBool, Token,
    },
};

/// Possible arguments to the `#[spl_program_error]` attribute
pub struct SplProgramErrorArgs {
    /// Whether to hash the error codes using `solana_program::hash`
    /// or to use the default error code assigned by `num_traits`.
    pub hash_error_codes: bool,
}

impl Parse for SplProgramErrorArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self {
                hash_error_codes: false,
            });
        }
        match SplProgramErrorArgParser::parse(input)? {
            SplProgramErrorArgParser::HashErrorCodes { value, .. } => Ok(Self {
                hash_error_codes: value.value,
            }),
        }
    }
}

/// Parser for args to the `#[spl_program_error]` attribute
/// ie. `#[spl_program_error(hash_error_codes = true)]`
enum SplProgramErrorArgParser {
    HashErrorCodes {
        _ident: Ident,
        _equals_sign: Token![=],
        value: LitBool,
        _comma: Option<Comma>,
    },
}

impl Parse for SplProgramErrorArgParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _ident = input.parse::<Ident>()?;
        let _equals_sign = input.parse::<Token![=]>()?;
        let value = input.parse::<LitBool>()?;
        let _comma: Option<Comma> = input.parse().unwrap_or(None);
        Ok(Self::HashErrorCodes {
            _ident,
            _equals_sign,
            value,
            _comma,
        })
    }
}
