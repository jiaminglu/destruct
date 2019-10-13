extern crate proc_macro;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::export::TokenStream2;
use syn::punctuated;
use syn::{parse2, Data, DeriveInput, Field, Fields};

fn get_destruct_type(fields: &mut punctuated::Iter<Field>) -> proc_macro2::TokenStream {
    match fields.next() {
        Some(head_field) => {
            let head = head_field.ty.clone();
            let tail = get_destruct_type(fields);
            quote! {
                destruct_lib::DestructField<#head, #tail>
            }
        }
        None => {
            quote! {
                destruct_lib::DestructEnd
            }
        }
    }
}

fn get_destruct_from(fields: &mut punctuated::Iter<Field>) -> proc_macro2::TokenStream {
    match fields.next() {
        Some(head_field) => {
            let head = head_field.ident.clone().unwrap();
            let tail = get_destruct_from(fields);
            quote! {
                destruct_lib::DestructField {
                    head: t.#head,
                    tail: #tail
                }
            }
        }
        None => {
            quote! {
                destruct_lib::DestructEnd
            }
        }
    }
}

fn get_destruct_into(fields: &mut punctuated::Iter<Field>) -> proc_macro2::TokenStream {
    let mut acc = quote! { . };
    let mut tokens = TokenStream2::new();
    for field in fields {
        let name = field.ident.clone().unwrap();
        tokens.extend(quote! {
            #name: self.fields #acc head,
        });
        acc = quote! { #acc tail . }
    }
    tokens
}

#[proc_macro_derive(Destruct)]
pub fn derive_destruct(input: TokenStream) -> TokenStream {
    let input = proc_macro2::TokenStream::from(input);
    let input: DeriveInput = parse2(input).unwrap();
    let name = input.ident;

    let fields = match input.data {
        Data::Struct(s) => match s.fields {
            Fields::Named(named) => named,
            _ => panic!("derive Destruct supports only named struct"),
        },
        _ => panic!("derive Destruct supports only structs"),
    };
    let destruct_type = get_destruct_type(&mut fields.named.iter());
    let destruct_from = get_destruct_from(&mut fields.named.iter());
    let destruct_into = get_destruct_into(&mut fields.named.iter());

    // Return the generated impl
    let output = quote! {
        impl From<#name> for DestructBegin<#destruct_type> {
            fn from(t: #name) -> Self {
                DestructBegin { fields: #destruct_from }
            }
        }

        impl Into<#name> for DestructBegin<
            #destruct_type
        > {
            fn into(self) -> #name {
                #name { #destruct_into }
            }
        }

        impl Destruct for #name {
            type DestructType = DestructBegin<#destruct_type>;

            fn destruct(self) -> Self::DestructType {
                self.into()
            }

            fn construct(d: Self::DestructType) -> Self {
                d.into()
            }
        }
    };
    proc_macro::TokenStream::from(output)
}
