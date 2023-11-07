//! Derive macro library for the `spl-discriminator` library

#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

extern crate proc_macro;

use {
    proc_macro::TokenStream, quote::ToTokens, spl_discriminator_syn::SplDiscriminateBuilder,
    syn::parse_macro_input,
};

/// Derive macro library to implement the `SplDiscriminate` trait
/// on an enum or struct
#[proc_macro_derive(SplDiscriminate, attributes(discriminator_hash_input))]
pub fn spl_discriminator(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as SplDiscriminateBuilder)
        .to_token_stream()
        .into()
}
