//! Token parsing and generating library for the `spl-discriminator` library

#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

mod error;
pub mod parser;

use {
    crate::parser::parse_hash_input,
    proc_macro2::{Span, TokenStream},
    quote::{quote, ToTokens},
    solana_program::hash,
    syn::{ext::IdentExt, parse::Parse, Attribute, Ident, LitByteStr, Token, Visibility},
};

/// "Builder" struct to implement the `SplDiscriminate` trait
/// on an enum or struct
pub struct SplDiscriminateBuilder {
    /// The struct/enum identifier
    pub ident: Ident,
    /// The TLV hash_input
    pub hash_input: String,
}

impl Parse for SplDiscriminateBuilder {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let attrs = input.call(Attribute::parse_outer)?;
        let hash_input = parse_hash_input(&attrs)
            .map_err(|e| syn::Error::new(input.span(), format!("Failed to parse item: {}", e)))?;
        if input.peek(Token![pub]) {
            input.parse::<Visibility>()?;
        }
        if input.peek(Token![struct]) ||input.peek(Token![enum]) {
            input.call(Ident::parse_any)?;
        }
        let ident = input.parse::<Ident>()?;
        // just consume the rest
        input.step(|cursor| {
            let mut rest = *cursor;
            while let Some((_, next)) = rest.token_tree() {
                rest = next;
            }
            Ok(((), rest))
        })?;
        Ok(Self { ident, hash_input })
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
        let bytes = get_discriminator_bytes(&builder.hash_input);
        quote! {
            impl spl_discriminator::discriminator::SplDiscriminate for #ident {
                const SPL_DISCRIMINATOR: spl_discriminator::discriminator::ArrayDiscriminator
                    = spl_discriminator::discriminator::ArrayDiscriminator::new(*#bytes);
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
