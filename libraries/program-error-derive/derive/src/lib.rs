extern crate proc_macro;

mod program_error;

use proc_macro::TokenStream;

/// Proc macro attribute to turn your enum into a Solana Program Error
#[proc_macro_attribute]
pub fn solana_program_error(_: TokenStream, input: TokenStream) -> TokenStream {
    program_error::program_error(input)
}
