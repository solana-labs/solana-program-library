//! Parser for the `syn` crate to parse the
//! `#[discriminator_hash_input("...")]` attribute

use {
    crate::error::SplDiscriminateError,
    syn::{
        parse::{Parse, ParseStream},
        token::Comma,
        Attribute, LitStr,
    },
};

/// Struct used for `syn` parsing of the hash_input attribute
/// #[discriminator_hash_input("...")]
struct HashInputValueParser {
    value: LitStr,
    _comma: Option<Comma>,
}

impl Parse for HashInputValueParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let value: LitStr = input.parse()?;
        let _comma: Option<Comma> = input.parse().unwrap_or(None);
        Ok(HashInputValueParser { value, _comma })
    }
}

/// Parses the hash_input from the `#[discriminator_hash_input("...")]`
/// attribute
pub fn parse_hash_input(attrs: &[Attribute]) -> Result<String, SplDiscriminateError> {
    match attrs
        .iter()
        .find(|a| a.path().is_ident("discriminator_hash_input"))
    {
        Some(attr) => {
            let parsed_args = attr
                .parse_args::<HashInputValueParser>()
                .map_err(|_| SplDiscriminateError::HashInputAttributeParseError)?;
            Ok(parsed_args.value.value())
        }
        None => Err(SplDiscriminateError::HashInputAttributeNotProvided),
    }
}
