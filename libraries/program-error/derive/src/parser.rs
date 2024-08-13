//! Token parsing

use {
    proc_macro2::{Ident, Span, TokenStream},
    quote::quote,
    syn::{
        parse::{Parse, ParseStream},
        token::Comma,
        LitInt, LitStr, Token,
    },
};

/// Possible arguments to the `#[spl_program_error]` attribute
pub struct SplProgramErrorArgs {
    /// Whether to hash the error codes using `solana_program::hash`
    /// or to use the default error code assigned by `num_traits`.
    pub hash_error_code_start: Option<u32>,
    /// Crate to use for solana_program
    pub import: SolanaProgram,
}

/// Struct representing the path to a `solana_program` crate, which may be
/// renamed or otherwise.
pub struct SolanaProgram {
    import: Ident,
    explicit: bool,
}
impl quote::ToTokens for SolanaProgram {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.import.to_tokens(tokens);
    }
}
impl SolanaProgram {
    pub fn wrap(&self, output: TokenStream) -> TokenStream {
        if self.explicit {
            output
        } else {
            anon_const_trick(output)
        }
    }
}
impl Default for SolanaProgram {
    fn default() -> Self {
        Self {
            import: Ident::new("_solana_program", Span::call_site()),
            explicit: false,
        }
    }
}

impl Parse for SplProgramErrorArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut hash_error_code_start = None;
        let mut import = None;
        while !input.is_empty() {
            match SplProgramErrorArgParser::parse(input)? {
                SplProgramErrorArgParser::HashErrorCodes { value, .. } => {
                    hash_error_code_start = Some(value.base10_parse::<u32>()?);
                }
                SplProgramErrorArgParser::SolanaProgramCrate { value, .. } => {
                    import = Some(SolanaProgram {
                        import: value.parse()?,
                        explicit: true,
                    });
                }
            }
        }
        Ok(Self {
            hash_error_code_start,
            import: import.unwrap_or(SolanaProgram::default()),
        })
    }
}

/// Parser for args to the `#[spl_program_error]` attribute
/// ie. `#[spl_program_error(hash_error_code_start = 1275525928)]`
enum SplProgramErrorArgParser {
    HashErrorCodes {
        _equals_sign: Token![=],
        value: LitInt,
        _comma: Option<Comma>,
    },
    SolanaProgramCrate {
        _equals_sign: Token![=],
        value: LitStr,
        _comma: Option<Comma>,
    },
}

impl Parse for SplProgramErrorArgParser {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident = input.parse::<Ident>()?;
        match ident.to_string().as_str() {
            "hash_error_code_start" => {
                let _equals_sign = input.parse::<Token![=]>()?;
                let value = input.parse::<LitInt>()?;
                let _comma: Option<Comma> = input.parse().unwrap_or(None);
                Ok(Self::HashErrorCodes {
                    _equals_sign,
                    value,
                    _comma,
                })
            }
            "solana_program" => {
                let _equals_sign = input.parse::<Token![=]>()?;
                let value = input.parse::<LitStr>()?;
                let _comma: Option<Comma> = input.parse().unwrap_or(None);
                Ok(Self::SolanaProgramCrate {
                    _equals_sign,
                    value,
                    _comma,
                })
            }
            _ => Err(input.error("Expected argument 'hash_error_code_start' or 'solana_program'")),
        }
    }
}

// Within `exp`, you can bring things into scope with `extern crate`.
//
// We don't want to assume that `solana_program::` is in scope - the user may
// have imported it under a different name, or may have imported it in a
// non-toplevel module (common when putting impls behind a feature gate).
//
// Solution: let's just generate `extern crate solana_program as
// _solana_program` and then refer to `_solana_program` in the derived code.
// However, macros are not allowed to produce `extern crate` statements at the
// toplevel.
//
// Solution: let's generate `mod _impl_foo` and import solana_program within
// that.  However, now we lose access to private members of the surrounding
// module.  This is a problem if, for example, we're deriving for a newtype,
// where the inner type is defined in the same module, but not exported.
//
// Solution: use the anonymous const trick.  For some reason, `extern crate`
// statements are allowed here, but everything from the surrounding module is in
// scope.  This trick is taken from serde and num_traits.
fn anon_const_trick(exp: TokenStream) -> TokenStream {
    quote! {
        const _: () = {
            extern crate solana_program as _solana_program;
            #exp
        };
    }
}
