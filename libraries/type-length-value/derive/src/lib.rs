//! Derive macro library for the `spl-type-length-value` library

#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

extern crate proc_macro;

use proc_macro::TokenStream;
use quote::ToTokens;
use spl_type_length_value_syn::TlvBuilder;
use syn::parse_macro_input;

/// Derive macro library to generate trait implementations and
/// types for TLV
#[proc_macro_derive(SplTlv, attributes(tlv_namespace))]
pub fn spl_tlv(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as TlvBuilder)
        .to_token_stream()
        .into()
}
