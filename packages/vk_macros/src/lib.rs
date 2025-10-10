use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields, parse_macro_input};

#[proc_macro_derive(EnumCount)]
pub fn enum_count_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident;

    let count = if let Data::Enum(data_enum) = input.data {
        data_enum.variants.len()
    } else {
        return syn::Error::new_spanned(
            enum_name,
            "#[derive(EnumCount)] can only be used with enums",
        )
        .to_compile_error()
        .into();
    };

    let expanded = quote! {
        impl #enum_name {
            pub const COUNT: usize = #count;
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(Vector)]
pub fn derive_vector(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident;

    let data = match input.data {
        Data::Enum(data_enum) => data_enum,
        _ => {
            return syn::Error::new_spanned(
                enum_name,
                "#[derive(Vector)] can only be derived for enums",
            )
            .to_compile_error()
            .into();
        }
    };

    let variants = data.variants.into_iter().map(|variant| {
        let variant_ident = variant.ident;
        match variant.fields {
            Fields::Unit => {
                quote! { #enum_name::#variant_ident }
            }
            Fields::Unnamed(fields) => {
                let field_defaults =
                    (0..fields.unnamed.len()).map(|_| quote! { Default::default() });
                quote! { #enum_name::#variant_ident(#(#field_defaults),*) }
            }
            Fields::Named(fields) => {
                let field_inits = fields.named.iter().map(|field| {
                    let field_name = field.ident.as_ref().unwrap();
                    quote! { #field_name: Default::default() }
                });
                quote! { #enum_name::#variant_ident { #(#field_inits),* } }
            }
        }
    });

    let expanded = quote! {
        impl #enum_name {
            pub fn vector() -> Vec<Self> {
                vec![
                    #(#variants),*
                ]
            }
        }
    };

    TokenStream::from(expanded)
}
#[proc_macro_derive(ToString)]
pub fn to_string_enum_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let enum_name = input.ident;

    let data_enum = match input.data {
        Data::Enum(data_enum) => data_enum,
        _ => {
            return syn::Error::new_spanned(enum_name, "ToString can only be derived for enums")
                .to_compile_error()
                .into();
        }
    };

    let arms = data_enum.variants.iter().map(|variant| {
        let variant_ident = &variant.ident;
        let variant_name = variant_ident.to_string();
        match variant.fields {
            Fields::Unit => {
                quote! {
                    #enum_name::#variant_ident => write!(f, "{}", #variant_name),
                }
            }
            _ => syn::Error::new_spanned(variant_ident, "ToStringEnum only supports unit variants")
                .to_compile_error(),
        }
    });

    let expanded = quote! {
        impl std::fmt::Display for #enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#arms)*
                }
            }
        }
    };

    TokenStream::from(expanded)
}
