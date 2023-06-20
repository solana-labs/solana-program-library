//! Token parsing and generating library for the `spl-discriminator` library

#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

mod error;
pub mod parser;

use {
    crate::{error::HasDiscriminatorError, parser::parse_hash_input},
    proc_macro2::{Span, TokenStream},
    quote::{quote, ToTokens},
    solana_program::hash,
    syn::{parse::Parse, Ident, Item, ItemEnum, ItemStruct, LitByteStr},
};

/// "Builder" struct to implement the `HasDiscriminator` trait
/// on an enum or struct
#[derive(Debug)]
pub struct HasDiscriminatorBuilder {
    /// The struct/enum identifier
    pub ident: Ident,
    /// The TLV hash_input
    pub hash_input: String,
}

impl TryFrom<ItemEnum> for HasDiscriminatorBuilder {
    type Error = HasDiscriminatorError;

    fn try_from(item_enum: ItemEnum) -> Result<Self, Self::Error> {
        let ident = item_enum.ident;
        let hash_input = parse_hash_input(&item_enum.attrs)?;
        Ok(Self { ident, hash_input })
    }
}

impl TryFrom<ItemStruct> for HasDiscriminatorBuilder {
    type Error = HasDiscriminatorError;

    fn try_from(item_struct: ItemStruct) -> Result<Self, Self::Error> {
        let ident = item_struct.ident;
        let hash_input = parse_hash_input(&item_struct.attrs)?;
        Ok(Self { ident, hash_input })
    }
}

impl Parse for HasDiscriminatorBuilder {
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

impl ToTokens for HasDiscriminatorBuilder {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend::<TokenStream>(self.into());
    }
}

impl From<&HasDiscriminatorBuilder> for TokenStream {
    fn from(builder: &HasDiscriminatorBuilder) -> Self {
        let ident = &builder.ident;
        let bytes = get_discriminator_bytes(&builder.hash_input);
        quote! {
            impl spl_discriminator::discriminator::HasDiscriminator for #ident {
                const SPL_DISCRIMINATOR: spl_discriminator::discriminator::Discriminator = spl_discriminator::discriminator::Discriminator::new(*#bytes);
            }
        }
    }
}

/// Returns the bytes for the TLV hash_input discriminator
fn get_discriminator_bytes(hash_input: &str) -> LitByteStr {
    LitByteStr::new(
        &hash::hashv(&[hash_input.as_bytes()]).to_bytes()[..8],
        Span::call_site(),
    )
}
