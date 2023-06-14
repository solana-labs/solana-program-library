//! Token parsing and generating library for the `type-length-value` library

#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

mod error;
pub mod parser;

use convert_case::{Case, Casing};
use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use solana_program::hash;
use syn::{parse::Parse, Ident, Item, ItemEnum, ItemStruct, LitByteStr};

use crate::error::SplTlvError;
use crate::parser::parse_tlv_namespace;

/// "Builder" struct to implement the TLV traits and
/// types on an enum or struct
#[derive(Debug)]
pub struct TlvBuilder {
    /// The struct/enum identifier
    pub ident: Ident,
    /// The TLV namespace
    pub namespace: String,
}

impl TryFrom<ItemEnum> for TlvBuilder {
    type Error = SplTlvError;

    fn try_from(item_enum: ItemEnum) -> Result<Self, Self::Error> {
        let ident = item_enum.ident;
        let namespace = parse_tlv_namespace(&item_enum.attrs)?;
        Ok(Self { ident, namespace })
    }
}

impl TryFrom<ItemStruct> for TlvBuilder {
    type Error = SplTlvError;

    fn try_from(item_struct: ItemStruct) -> Result<Self, Self::Error> {
        let ident = item_struct.ident;
        let namespace = parse_tlv_namespace(&item_struct.attrs)?;
        Ok(Self { ident, namespace })
    }
}

impl Parse for TlvBuilder {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let item = Item::parse(input)?;
        match item {
            Item::Enum(item_enum) => item_enum.try_into(),
            Item::Struct(item_struct) => item_struct.try_into(),
            _ => {
                return Err(syn::Error::new(
                    Span::call_site(),
                    "Only enums and structs are supported",
                ))
            }
        }
        .map_err(|e| {
            syn::Error::new(
                input.span(),
                format!("Failed to parse interface instructions: {}", e),
            )
        })
    }
}

impl ToTokens for TlvBuilder {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend::<TokenStream>(self.into());
    }
}

impl From<&TlvBuilder> for TokenStream {
    fn from(builder: &TlvBuilder) -> Self {
        let ident = &builder.ident;
        let (discriminator_name, discriminator_slice_name) = get_discriminator_const_names(ident);
        let bytes = get_discriminator_bytes(&builder.namespace);
        quote! {
            impl spl_type_length_value::discriminator::TlvDiscriminator for #ident {
                const TLV_DISCRIMINATOR: spl_type_length_value::discriminator::Discriminator = spl_type_length_value::discriminator::Discriminator::new(#discriminator_name);
            }
            const #discriminator_name: [u8; spl_type_length_value::discriminator::Discriminator::LENGTH] = *#bytes;
            const #discriminator_slice_name: &[u8] = &#discriminator_name;
        }
    }
}

/// Builds the constant variable name for the TLV discriminator bytes
/// ie: `INITIALIZE_DISCRIMINATOR: [u8; Discriminator::LENGTH]`
/// and the constant variable name for the TLV discriminator slice
/// ie: `INITIALIZE_DISCRIMINATOR_SLICE: &[u8] = &INITIALIZE_DISCRIMINATOR`
fn get_discriminator_const_names(ident: &Ident) -> (Ident, Ident) {
    let ident_upper = ident.to_string().to_case(Case::UpperSnake);
    let discriminator_name = format!("{}_DISCRIMINATOR", ident_upper);
    let discriminator_slice_name = format!("{}_DISCRIMINATOR_SLICE", ident_upper);
    (
        Ident::new(&discriminator_name, ident.span()),
        Ident::new(&discriminator_slice_name, ident.span()),
    )
}

/// Returns the bytes for the TLV namespace discriminator
fn get_discriminator_bytes(namespace: &str) -> LitByteStr {
    LitByteStr::new(
        &hash::hashv(&[namespace.as_bytes()]).to_bytes()[..8],
        Span::call_site(),
    )
}
