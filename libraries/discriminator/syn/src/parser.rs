//! Parser for the `syn` crate to parse the
//! `#[discriminator_namespace("...")]` attribute

use syn::{
    parse::{Parse, ParseStream},
    token::Comma,
    Attribute, LitStr,
};

use crate::error::SplDiscriminatorError;

/// Struct used for `syn` parsing of the namespace attribute
/// #[discriminator_namespace("...")]
struct NamespaceValueParser {
    value: LitStr,
    _comma: Option<Comma>,
}

impl Parse for NamespaceValueParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let value: LitStr = input.parse()?;
        let _comma: Option<Comma> = input.parse().unwrap_or(None);
        Ok(NamespaceValueParser { value, _comma })
    }
}

/// Parses the namespace from the `#[discriminator_namespace("...")]` attribute
pub fn parse_namespace(attrs: &[Attribute]) -> Result<String, SplDiscriminatorError> {
    match attrs
        .iter()
        .find(|a| a.path().is_ident("discriminator_namespace"))
    {
        Some(attr) => {
            let parsed_args = attr
                .parse_args::<NamespaceValueParser>()
                .map_err(|_| SplDiscriminatorError::NamespaceAttributeParseError)?;
            Ok(parsed_args.value.value())
        }
        None => Err(SplDiscriminatorError::NamespaceAttributeNotProvided),
    }
}
