//! The actual token generator for the macro
use {
    proc_macro2::{Span, TokenStream},
    quote::{quote, ToTokens},
    syn::{parse::Parse, Generics, Ident, Item, ItemEnum, ItemStruct, WhereClause},
};

pub struct SplBorshVariableLenPackBuilder {
    /// The struct/enum identifier
    pub ident: Ident,
    /// The item's generic arguments (if any)
    pub generics: Generics,
    /// The item's where clause for generics (if any)
    pub where_clause: Option<WhereClause>,
}

impl TryFrom<ItemEnum> for SplBorshVariableLenPackBuilder {
    type Error = syn::Error;

    fn try_from(item_enum: ItemEnum) -> Result<Self, Self::Error> {
        let ident = item_enum.ident;
        let where_clause = item_enum.generics.where_clause.clone();
        let generics = item_enum.generics;
        Ok(Self {
            ident,
            generics,
            where_clause,
        })
    }
}

impl TryFrom<ItemStruct> for SplBorshVariableLenPackBuilder {
    type Error = syn::Error;

    fn try_from(item_struct: ItemStruct) -> Result<Self, Self::Error> {
        let ident = item_struct.ident;
        let where_clause = item_struct.generics.where_clause.clone();
        let generics = item_struct.generics;
        Ok(Self {
            ident,
            generics,
            where_clause,
        })
    }
}

impl Parse for SplBorshVariableLenPackBuilder {
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

impl ToTokens for SplBorshVariableLenPackBuilder {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        tokens.extend::<TokenStream>(self.into());
    }
}

impl From<&SplBorshVariableLenPackBuilder> for TokenStream {
    fn from(builder: &SplBorshVariableLenPackBuilder) -> Self {
        let ident = &builder.ident;
        let generics = &builder.generics;
        let where_clause = &builder.where_clause;
        quote! {
            impl #generics spl_type_length_value::variable_len_pack::VariableLenPack for #ident #generics #where_clause {
                fn pack_into_slice(&self, dst: &mut [u8]) -> Result<(), spl_type_length_value::solana_program::program_error::ProgramError> {
                    borsh::to_writer(&mut dst[..], self).map_err(Into::into)
                }

                fn unpack_from_slice(src: &[u8]) -> Result<Self, spl_type_length_value::solana_program::program_error::ProgramError> {
                    solana_program::borsh1::try_from_slice_unchecked(src).map_err(Into::into)
                }

                fn get_packed_len(&self) -> Result<usize, spl_type_length_value::solana_program::program_error::ProgramError> {
                    solana_program::borsh1::get_instance_packed_len(self).map_err(Into::into)
                }
            }
        }
    }
}
