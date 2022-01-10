use proc_macro::TokenStream;
use quote::quote;
use syn::{self, DeriveInput, Ident};
use uuid::Uuid;

/// Allows an enum with variants to be used as a [`Nametag`][acidalia_core::Nametag].
#[proc_macro_derive(Nametag)]
pub fn nametag_derive(input: TokenStream) -> TokenStream {
    let ast: DeriveInput = syn::parse(input).unwrap();
    let ident = ast.ident.clone();

    if let syn::Data::Enum(en) = ast.data {
        let mut idents: Vec<Ident> = vec![];
        let mut vals: Vec<u128> = vec![];
        for variant in &en.variants {
            idents.push(variant.ident.clone());
            vals.push(Uuid::new_v4().as_u128());
        }

        let out = quote! {
            impl Nametag for #ident {
                fn tag(self) -> u128 {
                    match self {
                        #(Self::#idents =>  { #vals }),*
                    }
                }
            }
        };

        return out.into();
    }

    return TokenStream::new();
}

// #[proc_macro_derive(FnAlias)]
// pub fn fnalias_derive(input: TokenStream) -> TokenStream {
//     let ast: DeriveInput = syn::parse(input).unwrap();
//     TokenStream::new()
// }
