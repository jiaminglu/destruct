extern crate proc_macro;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::export::TokenStream2;
use syn::punctuated;
use syn::{parse2, Data, DeriveInput, Field, Fields, Ident, LitStr};

fn get_destruct_type(
    name: &Ident,
    fields: &mut punctuated::Iter<Field>,
) -> proc_macro2::TokenStream {
    match fields.next() {
        Some(head_field) => {
            let head_name = head_field.ident.clone().unwrap();
            let metadata_name = format_ident!("_destruct_{}_field_{}_meta", name, head_name);
            let head = head_field.ty.clone();
            let tail = get_destruct_type(name, fields);
            quote! {
                destruct_lib::DestructField<#head, #tail, #metadata_name>
            }
        }
        None => {
            let metadata_name = format_ident!("_destruct_{}_meta", name);
            quote! {
                destruct_lib::DestructEnd<#metadata_name>
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
                destruct_lib::DestructField::new(t.#head, #tail)
            }
        }
        None => {
            quote! {
                destruct_lib::DestructEnd::new()
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

fn get_destruct_field_meta(
    name: &Ident,
    fields: &mut punctuated::Iter<Field>,
) -> proc_macro2::TokenStream {
    let mut tokens = TokenStream2::new();
    for field in fields {
        let field_name = field.ident.clone().unwrap();
        let field_meta_name = format_ident!("_destruct_{}_field_{}_meta", name, field_name);
        let s = format!("{}", name);
        let lit_name = LitStr::new(s.as_str(), name.span());
        let s = format!("{}", field_name);
        let field_lit_name = LitStr::new(s.as_str(), field_name.span());
        tokens.extend(quote! {
            #[allow(non_camel_case_types)]
            struct #field_meta_name;

            impl DestructMetadata for #field_meta_name {
                fn struct_name() -> &'static str {
                    #lit_name
                }
            }
            impl DestructFieldMetadata for #field_meta_name {
                fn head_name() -> &'static str {
                    #field_lit_name
                }
            }
        });
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
    let destruct_type = get_destruct_type(&name, &mut fields.named.iter());
    let destruct_from = get_destruct_from(&mut fields.named.iter());
    let destruct_into = get_destruct_into(&mut fields.named.iter());
    let destruct_field_meta = get_destruct_field_meta(&name, &mut fields.named.iter());

    let destruct_meta_name = format_ident!("_destruct_{}_meta", name);
    let s = format!("{}", name);
    let lit_name = LitStr::new(s.as_str(), name.span());

    // Return the generated impl
    let output = quote! {
        impl From<#name> for DestructBegin<#destruct_type, #destruct_meta_name> {
            fn from(t: #name) -> Self {
                DestructBegin::new(#destruct_from)
            }
        }

        #[allow(non_camel_case_types)]
        struct #destruct_meta_name;

        impl DestructMetadata for #destruct_meta_name {
            fn struct_name() -> &'static str {
                #lit_name
            }
        }

        #destruct_field_meta

        impl Into<#name> for DestructBegin<#destruct_type, #destruct_meta_name> {
            fn into(self) -> #name {
                #name { #destruct_into }
            }
        }

        impl Destruct for #name {
            type DestructType = DestructBegin<#destruct_type, #destruct_meta_name>;

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
