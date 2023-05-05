//! The actual token generator for the macro
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Error, LitStr, Variant};

/// The main function that produces the tokens required to turn your
/// error enum into a Solana Program Error
pub fn program_error(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = &input.ident;
    let data = &input.data;

    // Error if its not an enum
    let variants = if let Data::Enum(enum_data) = data {
        &enum_data.variants
    } else {
        return Error::new(Span::call_site(), "Expected an enum")
            .to_compile_error()
            .into();
    };

    // Build the match arms for `PrintProgramError`
    let ppe_match_arms = variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        let error_msg = get_error_message(variant)
            .unwrap_or_else(|| String::from("Unknown custom program error"));
        quote! {
            #ident::#variant_ident => {
                solana_program::msg!(#error_msg)
            }
        }
    });

    quote! {
        #[derive(Clone, Debug, Eq, thiserror::Error, num_derive::FromPrimitive, PartialEq)]
        #input

        impl From<#ident> for solana_program::program_error::ProgramError {
            fn from(e: #ident) -> Self {
                solana_program::program_error::ProgramError::Custom(e as u32)
            }
        }

        impl<T> solana_program::decode_error::DecodeError<T> for #ident {
            fn type_of() -> &'static str {
                stringify!(#ident)
            }
        }

        impl solana_program::program_error::PrintProgramError for #ident {
            fn print<E>(&self)
            where
                E: 'static
                    + std::error::Error
                    + solana_program::decode_error::DecodeError<E>
                    + solana_program::program_error::PrintProgramError
                    + num_traits::FromPrimitive,
            {
                match self {
                    #(#ppe_match_arms),*
                }
            }
        }
    }
    .into()
}

/// Helper to parse out the string literal from the `#[error(..)]` attribute
fn get_error_message(variant: &Variant) -> Option<String> {
    let attrs = &variant.attrs;

    for attr in attrs {
        if attr.path.is_ident("error") {
            if let Ok(lit_str) = attr.parse_args::<LitStr>() {
                return Some(lit_str.value());
            }
        }
    }

    None
}
