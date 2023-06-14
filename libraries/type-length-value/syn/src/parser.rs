//! Parser for the `syn` crate to parse the
//! `#[tlv_namespace = "..."]` attribute

use syn::{
    parse::{Parse, ParseStream},
    token::Comma,
    Attribute, LitStr,
};

use crate::error::SplTlvError;

/// Struct used for `syn` parsing of the TLV namespace attribute
/// #[tlv_namespace = "..."]
struct TlvNamespaceValueParser {
    value: LitStr,
    _comma: Option<Comma>,
}

impl Parse for TlvNamespaceValueParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let value: LitStr = input.parse()?;
        let _comma: Option<Comma> = input.parse().unwrap_or(None);
        Ok(TlvNamespaceValueParser { value, _comma })
    }
}

/// Parses the TLV namespace from the `#[tlv_namespace = "...")]` attribute
pub fn parse_tlv_namespace(attrs: &[Attribute]) -> Result<String, SplTlvError> {
    match attrs.iter().find(|a| a.path().is_ident("tlv_namespace")) {
        Some(attr) => {
            let parsed_args = attr
                .parse_args::<TlvNamespaceValueParser>()
                .map_err(|_| SplTlvError::TlvNamespaceAttributeParseError)?;
            Ok(parsed_args.value.value())
        }
        None => Err(SplTlvError::TlvNamespaceAttributeNotProvided),
    }
}
