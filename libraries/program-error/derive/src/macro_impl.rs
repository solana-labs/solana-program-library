//! The actual token generator for the macro

use {
    crate::parser::{SolanaProgram, SplProgramErrorArgs},
    proc_macro2::Span,
    quote::quote,
    sha2::{Digest, Sha256},
    syn::{
        punctuated::Punctuated, token::Comma, Expr, ExprLit, Ident, ItemEnum, Lit, LitInt, LitStr,
        Token, Variant,
    },
};

const SPL_ERROR_HASH_NAMESPACE: &str = "spl_program_error";
const SPL_ERROR_HASH_MIN_VALUE: u32 = 7_000;

/// The type of macro being called, thus directing which tokens to generate
#[allow(clippy::enum_variant_names)]
pub enum MacroType {
    IntoProgramError {
        ident: Ident,
    },
    DecodeError {
        ident: Ident,
    },
    PrintProgramError {
        ident: Ident,
        variants: Punctuated<Variant, Comma>,
    },
    SplProgramError {
        args: SplProgramErrorArgs,
        item_enum: ItemEnum,
    },
}

impl MacroType {
    /// Generates the corresponding tokens based on variant selection
    pub fn generate_tokens(&mut self) -> proc_macro2::TokenStream {
        let default_solana_program = SolanaProgram::default();
        match self {
            Self::IntoProgramError { ident } => into_program_error(ident, &default_solana_program),
            Self::DecodeError { ident } => decode_error(ident, &default_solana_program),
            Self::PrintProgramError { ident, variants } => {
                print_program_error(ident, variants, &default_solana_program)
            }
            Self::SplProgramError { args, item_enum } => spl_program_error(args, item_enum),
        }
    }
}

/// Builds the implementation of
/// `Into<solana_program::program_error::ProgramError>` More specifically,
/// implements `From<Self> for solana_program::program_error::ProgramError`
pub fn into_program_error(ident: &Ident, import: &SolanaProgram) -> proc_macro2::TokenStream {
    let this_impl = quote! {
        impl From<#ident> for #import::program_error::ProgramError {
            fn from(e: #ident) -> Self {
                #import::program_error::ProgramError::Custom(e as u32)
            }
        }
    };
    import.wrap(this_impl)
}

/// Builds the implementation of `solana_program::decode_error::DecodeError<T>`
pub fn decode_error(ident: &Ident, import: &SolanaProgram) -> proc_macro2::TokenStream {
    let this_impl = quote! {
        impl<T> #import::decode_error::DecodeError<T> for #ident {
            fn type_of() -> &'static str {
                stringify!(#ident)
            }
        }
    };
    import.wrap(this_impl)
}

/// Builds the implementation of
/// `solana_program::program_error::PrintProgramError`
pub fn print_program_error(
    ident: &Ident,
    variants: &Punctuated<Variant, Comma>,
    import: &SolanaProgram,
) -> proc_macro2::TokenStream {
    let ppe_match_arms = variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        let error_msg = get_error_message(variant)
            .unwrap_or_else(|| String::from("Unknown custom program error"));
        quote! {
            #ident::#variant_ident => {
                #import::msg!(#error_msg)
            }
        }
    });
    let this_impl = quote! {
        impl #import::program_error::PrintProgramError for #ident {
            fn print<E>(&self)
            where
                E: 'static
                    + std::error::Error
                    + #import::decode_error::DecodeError<E>
                    + #import::program_error::PrintProgramError
                    + num_traits::FromPrimitive,
            {
                match self {
                    #(#ppe_match_arms),*
                }
            }
        }
    };
    import.wrap(this_impl)
}

/// Helper to parse out the string literal from the `#[error(..)]` attribute
fn get_error_message(variant: &Variant) -> Option<String> {
    let attrs = &variant.attrs;
    for attr in attrs {
        if attr.path().is_ident("error") {
            if let Ok(lit_str) = attr.parse_args::<LitStr>() {
                return Some(lit_str.value());
            }
        }
    }
    None
}

/// The main function that produces the tokens required to turn your
/// error enum into a Solana Program Error
pub fn spl_program_error(
    args: &SplProgramErrorArgs,
    item_enum: &mut ItemEnum,
) -> proc_macro2::TokenStream {
    if let Some(error_code_start) = args.hash_error_code_start {
        set_first_discriminant(item_enum, error_code_start);
    }

    let ident = &item_enum.ident;
    let variants = &item_enum.variants;
    let into_program_error = into_program_error(ident, &args.import);
    let decode_error = decode_error(ident, &args.import);
    let print_program_error = print_program_error(ident, variants, &args.import);

    quote! {
        #[repr(u32)]
        #[derive(Clone, Debug, Eq, thiserror::Error, num_derive::FromPrimitive, PartialEq)]
        #[num_traits = "num_traits"]
        #item_enum

        #into_program_error

        #decode_error

        #print_program_error
    }
}

/// This function adds a discriminant to the first enum variant based on the
/// hash of the `SPL_ERROR_HASH_NAMESPACE` constant, the enum name and variant
/// name.
/// It will then check to make sure the provided `hash_error_code_start` is
/// equal to the hash-produced `u32`.
///
/// See https://docs.rs/syn/latest/syn/struct.Variant.html
fn set_first_discriminant(item_enum: &mut ItemEnum, error_code_start: u32) {
    let enum_ident = &item_enum.ident;
    if item_enum.variants.is_empty() {
        panic!("Enum must have at least one variant");
    }
    let first_variant = &mut item_enum.variants[0];
    let discriminant = u32_from_hash(enum_ident);
    if discriminant == error_code_start {
        let eq = Token![=](Span::call_site());
        let expr = Expr::Lit(ExprLit {
            attrs: Vec::new(),
            lit: Lit::Int(LitInt::new(&discriminant.to_string(), Span::call_site())),
        });
        first_variant.discriminant = Some((eq, expr));
    } else {
        panic!(
            "Error code start value from hash must be {0}. Update your macro attribute to \
             `#[spl_program_error(hash_error_code_start = {0})]`.",
            discriminant
        );
    }
}

/// Hashes the `SPL_ERROR_HASH_NAMESPACE` constant, the enum name and variant
/// name and returns four middle bytes (13 through 16) as a u32.
fn u32_from_hash(enum_ident: &Ident) -> u32 {
    let hash_input = format!("{}:{}", SPL_ERROR_HASH_NAMESPACE, enum_ident);

    // We don't want our error code to start at any number below
    // `SPL_ERROR_HASH_MIN_VALUE`!
    let mut nonce: u32 = 0;
    loop {
        let mut hasher = Sha256::new_with_prefix(hash_input.as_bytes());
        hasher.update(nonce.to_le_bytes());
        let d = u32::from_le_bytes(
            hasher.finalize()[13..17]
                .try_into()
                .expect("Unable to convert hash to u32"),
        );
        if d >= SPL_ERROR_HASH_MIN_VALUE {
            return d;
        }
        nonce += 1;
    }
}
