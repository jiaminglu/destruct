extern crate proc_macro;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::export::TokenStream2;
use syn::punctuated;
use syn::punctuated::Punctuated;
use syn::{parse2, Data, DeriveInput, Field, Fields, Ident, LitStr};

struct FieldOrdered(Field, usize);

fn get_destruct_type(
    name: &Ident,
    fields: &mut std::slice::Iter<FieldOrdered>,
) -> proc_macro2::TokenStream {
    match fields.next() {
        Some(head_field) => {
            let head_name = head_field
                .0
                .ident
                .clone()
                .unwrap_or(format_ident!("unnamed_{}", head_field.1));
            let metadata_name = format_ident!("_destruct_{}_field_{}_meta", name, head_name);
            let head = head_field.0.ty.clone();
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

fn get_destruct_from(fields: &mut std::slice::Iter<FieldOrdered>) -> proc_macro2::TokenStream {
    match fields.next() {
        Some(head_field) => {
            let tail = get_destruct_from(fields);
            match head_field.0.ident.clone() {
                Some(head) => {
                    quote! {
                        destruct_lib::DestructField::new(t.#head, #tail)
                    }
                }
                None => {
                    let i = head_field.1;
                    quote! {
                        destruct_lib::DestructField::new(t.#i, #tail)
                    }
                }
            }
        }
        None => {
            quote! {
                destruct_lib::DestructEnd::new()
            }
        }
    }
}

fn get_destruct_into(
    name: &Ident,
    struct_is_named: bool,
    fields: &mut std::slice::Iter<FieldOrdered>,
) -> proc_macro2::TokenStream {
    let mut acc = quote! { . };
    let mut tokens = TokenStream2::new();
    for field in fields {
        match field.0.ident.clone() {
            Some(name) => {
                tokens.extend(quote! {
                    #name: self.fields #acc head,
                });
                acc = quote! { #acc tail . }
            }
            None => {
                tokens.extend(quote! {
                    self.fields #acc head,
                });
                acc = quote! { #acc tail . }
            }
        }
    }
    if struct_is_named {
        quote! {
            #name { #tokens }
        }
    } else {
        quote! {
            #name ( #tokens )
        }
    }
}

fn get_destruct_field_meta(
    name: &Ident,
    struct_is_named: bool,
    fields: &mut std::slice::Iter<FieldOrdered>,
) -> proc_macro2::TokenStream {
    let mut tokens = TokenStream2::new();
    for field in fields {
        let field_name = field
            .0
            .ident
            .clone()
            .unwrap_or(format_ident!("unnamed_{}", field.1));
        let field_meta_name = format_ident!("_destruct_{}_field_{}_meta", name, field_name);
        let s = format!("{}", name);
        let lit_name = LitStr::new(s.as_str(), name.span());
        let s = format!("{}", field_name);
        let field_lit_name = LitStr::new(s.as_str(), field_name.span());
        tokens.extend(quote! {
            #[allow(non_camel_case_types)]
            #[derive(Debug, PartialEq, Eq)]
            struct #field_meta_name;

            impl DestructMetadata for #field_meta_name {
                fn struct_name() -> &'static str {
                    #lit_name
                }
                fn named_fields() -> bool {
                    #struct_is_named
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

    let result = match input.data {
        Data::Struct(s) => {
            let struct_is_named;
            let fields = match s.fields {
                Fields::Named(named) => {
                    struct_is_named = true;
                    named
                        .named
                        .iter()
                        .enumerate()
                        .map(|(i, f)| FieldOrdered(f.clone(), i))
                        .collect()
                }
                Fields::Unnamed(unnamed) => {
                    struct_is_named = false;
                    unnamed
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(i, f)| FieldOrdered(f.clone(), i))
                        .collect()
                }
                Fields::Unit => {
                    struct_is_named = false;
                    Vec::new()
                }
            };
            derive_struct(name, struct_is_named, fields)
        },
        _ => panic!("derive Destruct supports only structs"),
    };
    proc_macro::TokenStream::from(result)
}

fn derive_struct(name: Ident, struct_is_named: bool, fields: Vec<FieldOrdered>) -> TokenStream2 {
    let destruct_type = get_destruct_type(&name, &mut fields.iter());
    let destruct_from = get_destruct_from(&mut fields.iter());
    let destruct_into = get_destruct_into(&name, struct_is_named, &mut fields.iter());
    let destruct_field_meta = get_destruct_field_meta(&name, struct_is_named, &mut fields.iter());

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
        #[derive(Debug, PartialEq, Eq)]
        struct #destruct_meta_name;

        impl DestructMetadata for #destruct_meta_name {
            fn struct_name() -> &'static str {
                #lit_name
            }
            fn named_fields() -> bool {
                #struct_is_named
            }
        }

        #destruct_field_meta

        impl Into<#name> for DestructBegin<#destruct_type, #destruct_meta_name> {
            fn into(self) -> #name {
                #destruct_into
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
    output
}
