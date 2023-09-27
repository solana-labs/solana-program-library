//! Token parsing and generating library for the `spl-discriminator` library

#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

mod error;
pub mod parser;

use {
    crate::{error::SplDiscriminateError, parser::parse_hash_input},
    proc_macro2::{Span, TokenStream},
    quote::{quote, ToTokens},
    sha2::{Digest, Sha256},
    syn::{parse::Parse, Generics, Ident, Item, ItemEnum, ItemStruct, LitByteStr, WhereClause},
};

/// "Builder" struct to implement the `SplDiscriminate` trait
/// on an enum or struct
pub struct SplDiscriminateBuilder {
    /// The struct/enum identifier
    pub ident: Ident,
    /// The item's generic arguments (if any)
    pub generics: Generics,
    /// The item's where clause for generics (if any)
    pub where_clause: Option<WhereClause>,
    /// The TLV hash_input
    pub hash_input: String,
}

impl TryFrom<ItemEnum> for SplDiscriminateBuilder {
    type Error = SplDiscriminateError;

    fn try_from(item_enum: ItemEnum) -> Result<Self, Self::Error> {
        let ident = item_enum.ident;
        let where_clause = item_enum.generics.where_clause.clone();
        let generics = item_enum.generics;
        let hash_input = parse_hash_input(&item_enum.attrs)?;
        Ok(Self {
            ident,
            generics,
            where_clause,
            hash_input,
        })
    }
}

impl TryFrom<ItemStruct> for SplDiscriminateBuilder {
    type Error = SplDiscriminateError;

    fn try_from(item_struct: ItemStruct) -> Result<Self, Self::Error> {
        let ident = item_struct.ident;
        let where_clause = item_struct.generics.where_clause.clone();
        let generics = item_struct.generics;
        let hash_input = parse_hash_input(&item_struct.attrs)?;
        Ok(Self {
            ident,
            generics,
            where_clause,
            hash_input,
        })
    }
}

impl Parse for SplDiscriminateBuilder {
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
        .map_err(|e| syn::Error::new(input.span(), format!("Failed to parse item: {}", e)))
    }
}

impl ToTokens for SplDiscriminateBuilder {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend::<TokenStream>(self.into());
    }
}

impl From<&SplDiscriminateBuilder> for TokenStream {
    fn from(builder: &SplDiscriminateBuilder) -> Self {
        let ident = &builder.ident;
        let generics = &builder.generics;
        let where_clause = &builder.where_clause;
        let bytes = get_discriminator_bytes(&builder.hash_input);
        quote! {
            impl #generics spl_discriminator::discriminator::SplDiscriminate for #ident #generics #where_clause {
                const SPL_DISCRIMINATOR: spl_discriminator::discriminator::ArrayDiscriminator
                    = spl_discriminator::discriminator::ArrayDiscriminator::new(*#bytes);
            }
        }
    }
}

/// Returns the bytes for the TLV hash_input discriminator
fn get_discriminator_bytes(hash_input: &str) -> LitByteStr {
    LitByteStr::new(
        &Sha256::digest(hash_input.as_bytes())[..8],
        Span::call_site(),
    )
}
