//! todo doc

mod builder;

use {
    builder::SplSerdeBuilder,
    proc_macro::TokenStream,
    syn::{parse_macro_input, DeriveInput},
};

#[proc_macro_attribute]
pub fn spl_serde(attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut spl_serde_builder = parse_macro_input!(attr as SplSerdeBuilder);
    let input = parse_macro_input!(input as DeriveInput);

    spl_serde_builder.expand(input)
}
