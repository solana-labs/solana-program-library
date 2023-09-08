use proc_macro::TokenStream;
use quote::quote;
use syn::{
    fold::{fold_derive_input, fold_variant, Fold},
    parse::{Parse, ParseStream},
    parse_quote, Attribute, DeriveInput, Field, LitStr, Variant,
};

pub(crate) struct SplSerdeBuilder {
    convert: Option<LitStr>,
    is_enum: bool,
    has_snake_case_enum_fields: bool,
}

impl SplSerdeBuilder {
    fn type_to_serde_with(type_name: &str) -> Option<&'static str> {
        match type_name {
            "COption<Pubkey>" => Some("coption_fromstr"),
            "DecryptableBalance" => Some("aeciphertext_fromstr"),
            "DecryptHandle" => Some("decrypthandle_fromstr"),
            "ElGamalPubkey" => Some("elgamalpubkey_fromstr"),
            "Pubkey" => Some("As::<DisplayFromStr>"),
            _ => None,
        }
    }

    fn should_skip_field(attrs: &[Attribute]) -> Option<usize> {
        attrs
            .iter()
            .position(|attr| attr.path().is_ident("spl_serde_skip"))
    }

    pub(crate) fn expand(&mut self, input: DeriveInput) -> TokenStream {
        let output = self.fold_derive_input(input);
        TokenStream::from(quote!(#output))
    }
}

impl Parse for SplSerdeBuilder {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut convert = None;
        if !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            if ident == "convert" {
                input.parse::<syn::Token![=]>()?;
                convert = Some(input.parse()?);
            }
        }
        Ok(SplSerdeBuilder {
            convert,
            is_enum: false,
            has_snake_case_enum_fields: false,
        })
    }
}

impl Fold for SplSerdeBuilder {
    fn fold_field(&mut self, mut field: Field) -> Field {
        if let Some(skip_attr_position) = SplSerdeBuilder::should_skip_field(&field.attrs) {
            field.attrs.remove(skip_attr_position); // remove #[spl_serde_skip] attribute if found
        } else {
            let field_ty = &field.ty;
            let field_ty_str = quote!(#field_ty).to_string();
            if let Some(serde_with) = SplSerdeBuilder::type_to_serde_with(field_ty_str.as_str()) {
                field
                    .attrs
                    .push(parse_quote! (#[serde(with = #serde_with)]));
            }

            if self.is_enum && !self.has_snake_case_enum_fields {
                if let Some(ident) = &field.ident {
                    self.has_snake_case_enum_fields = ident.to_string().contains('_');
                }
            }
        }

        field
    }

    fn fold_variant(&mut self, variant: Variant) -> Variant {
        self.is_enum = true;
        fold_variant(self, variant)
    }

    fn fold_derive_input(&mut self, input: DeriveInput) -> DeriveInput {
        let mut input = fold_derive_input(self, input);
        input
            .attrs
            .push(parse_quote!(#[derive(Serialize, Deserialize)]));

        if let Some(convert) = self.convert.as_ref() {
            input
                .attrs
                .push(parse_quote!(#[serde(from = #convert, into = #convert)]));
        }

        if self.has_snake_case_enum_fields {
            input.attrs.push(
                parse_quote!(#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]),
            );
        } else {
            input
                .attrs
                .push(parse_quote!(#[serde(rename_all = "camelCase")]));
        }

        input
    }
}
