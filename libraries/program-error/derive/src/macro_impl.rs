//! The actual token generator for the macro
use quote::quote;
use syn::{punctuated::Punctuated, token::Comma, Ident, ItemEnum, LitStr, Variant};

/// The type of macro being called, thus directing which tokens to generate
#[allow(clippy::enum_variant_names)]
pub enum MacroType {
    IntoProgramError,
    DecodeError,
    PrintProgramError,
    SplProgramError,
}

impl MacroType {
    /// Generates the corresponding tokens based on variant selection
    pub fn generate_tokens(&self, item_enum: ItemEnum) -> proc_macro2::TokenStream {
        match self {
            MacroType::IntoProgramError => into_program_error(&item_enum.ident),
            MacroType::DecodeError => decode_error(&item_enum.ident),
            MacroType::PrintProgramError => {
                print_program_error(&item_enum.ident, &item_enum.variants)
            }
            MacroType::SplProgramError => spl_program_error(item_enum),
        }
    }
}

/// Builds the implementation of `Into<solana_program::program_error::ProgramError>`
/// More specifically, implements `From<Self> for solana_program::program_error::ProgramError`
pub fn into_program_error(ident: &Ident) -> proc_macro2::TokenStream {
    quote! {
        impl From<#ident> for solana_program::program_error::ProgramError {
            fn from(e: #ident) -> Self {
                solana_program::program_error::ProgramError::Custom(e as u32)
            }
        }
    }
}

/// Builds the implementation of `solana_program::decode_error::DecodeError<T>`
pub fn decode_error(ident: &Ident) -> proc_macro2::TokenStream {
    quote! {
        impl<T> solana_program::decode_error::DecodeError<T> for #ident {
            fn type_of() -> &'static str {
                stringify!(#ident)
            }
        }
    }
}

/// Builds the implementation of `solana_program::program_error::PrintProgramError`
pub fn print_program_error(
    ident: &Ident,
    variants: &Punctuated<Variant, Comma>,
) -> proc_macro2::TokenStream {
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

/// The main function that produces the tokens required to turn your
/// error enum into a Solana Program Error
pub fn spl_program_error(input: ItemEnum) -> proc_macro2::TokenStream {
    let ident = &input.ident;
    let variants = &input.variants;
    let into_program_error = into_program_error(ident);
    let decode_error = decode_error(ident);
    let print_program_error = print_program_error(ident, variants);
    quote! {
        #[derive(Clone, Debug, Eq, thiserror::Error, num_derive::FromPrimitive, PartialEq)]
        #input

        #into_program_error

        #decode_error

        #print_program_error
    }
}
