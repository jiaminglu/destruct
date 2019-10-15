extern crate proc_macro;
#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::export::TokenStream2;
use syn::punctuated;
use syn::{parse2, Data, DeriveInput, Field, Fields, Ident, LitStr, Variant};

struct FieldOrdered(Field, usize);

#[derive(PartialEq, Eq)]
enum FieldType {
    Named,
    Unnamed,
    Unit,
}

fn convert_fields(fields: &Fields) -> (FieldType, Vec<FieldOrdered>) {
    let field_type;
    let fields = match fields {
        Fields::Named(named) => {
            field_type = FieldType::Named;
            named
                .named
                .iter()
                .enumerate()
                .map(|(i, f)| FieldOrdered(f.clone(), i))
                .collect()
        }
        Fields::Unnamed(unnamed) => {
            field_type = FieldType::Unnamed;
            unnamed
                .unnamed
                .iter()
                .enumerate()
                .map(|(i, f)| FieldOrdered(f.clone(), i))
                .collect()
        }
        Fields::Unit => {
            field_type = FieldType::Unit;
            Vec::new()
        }
    };
    (field_type, fields)
}

fn get_destruct_enum_type(
    name: &Ident,
    variants: &mut punctuated::Iter<Variant>,
) -> proc_macro2::TokenStream {
    match variants.next() {
        Some(variant) => {
            let vname = format_ident!("_destruct_enum_{}_variant_{}", name, variant.ident);
            let metadata_name =
                format_ident!("_destruct_enum_{}_variant_{}_meta", name, variant.ident);
            let field_struct_metadata_name = format_ident!(
                "_destruct__destruct_enum_{}_variant_{}_meta",
                name,
                variant.ident
            );
            let tail = get_destruct_enum_type(name, variants);
            let (_, fields) = convert_fields(&variant.fields);
            let destruct_type = get_destruct_type(&vname, &mut fields.iter());
            quote! {
                destruct_lib::DestructEnumVariant<destruct_lib::DestructBegin<#destruct_type, #field_struct_metadata_name>, #tail, #metadata_name>
            }
        }
        None => {
            let metadata_name = format_ident!("_destruct_enum_{}_meta", name);
            quote! {
                destruct_lib::DestructEnumEnd<#metadata_name>
            }
        }
    }
}
fn get_destruct_enum_from(
    name: &Ident,
    variants: &mut punctuated::Iter<Variant>,
) -> proc_macro2::TokenStream {
    match variants.next() {
        Some(variant) => {
            let ident = variant.ident.clone();
            let vname = format_ident!("_destruct_enum_{}_variant_{}", name, variant.ident);
            let (field_type, fields) = convert_fields(&variant.fields);
            let idents: Vec<Ident> = fields
                .iter()
                .map(|f| f.0.ident.clone().unwrap_or(format_ident!("field{}", f.1)))
                .collect();
            let variant_case = match field_type {
                FieldType::Named => {
                    quote! {
                        #name::#ident { #(#idents,)* } => destruct_lib::DestructEnumVariant::new_head((#vname { #(#idents,)* }).destruct())
                    }
                }
                FieldType::Unnamed => {
                    quote! {
                        #name::#ident ( #(#idents,)* ) => destruct_lib::DestructEnumVariant::new_head(#vname ( #(#idents,)* ).destruct())
                    }
                }
                FieldType::Unit => {
                    quote! {
                        #name::#ident => destruct_lib::DestructEnumVariant::new_head(#vname.destruct());
                    }
                }
            };
            let tail = get_destruct_enum_from(name, variants);
            quote! {
                #variant_case,
                other_case => destruct_lib::DestructEnumVariant::new_tail(match other_case { #tail })
            }
        }
        None => {
            quote! {
                _ => destruct_lib::DestructEnumEnd::new()
            }
        }
    }
}
fn get_destruct_enum_into(
    name: &Ident,
    variants: &mut punctuated::Iter<Variant>,
) -> proc_macro2::TokenStream {
    match variants.next() {
        Some(variant) => {
            let ident = variant.ident.clone();
            let (field_type, fields) = convert_fields(&variant.fields);
            let value_name = format_ident!("variant");
            let case = get_destruct_into_fields(
                &value_name,
                field_type == FieldType::Named,
                &mut fields.iter(),
            );
            let tail = get_destruct_enum_into(name, variants);
            quote! {
                destruct_lib::DestructEnumVariant::Head(variant, _) => #name::#ident #case,
                DestructEnumVariant::Tail(tail, _) => match tail { #tail }
            }
        }
        None => {
            quote! {
                _ => panic!("impossible")
            }
        }
    }
}

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
                    let i = proc_macro2::Literal::usize_unsuffixed(head_field.1);
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

fn get_destruct_into_fields(
    self_name: &Ident,
    struct_is_named: bool,
    fields: &mut std::slice::Iter<FieldOrdered>,
) -> proc_macro2::TokenStream {
    let mut acc = quote! { . };
    let mut tokens = TokenStream2::new();
    for field in fields {
        match field.0.ident.clone() {
            Some(name) => {
                tokens.extend(quote! {
                    #name: #self_name.fields #acc head,
                });
                acc = quote! { #acc tail . }
            }
            None => {
                tokens.extend(quote! {
                    #self_name.fields #acc head,
                });
                acc = quote! { #acc tail . }
            }
        }
    }
    if struct_is_named {
        quote! {
            { #tokens }
        }
    } else {
        quote! {
            ( #tokens )
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
            let (field_type, fields) = convert_fields(&s.fields);
            let s = format!("{}", name);
            let lit_name = LitStr::new(s.as_str(), name.span());
            derive_struct(name, lit_name, field_type == FieldType::Named, fields)
        }
        Data::Enum(e) => {
            let mut tt = TokenStream2::new();
            let s = format!("{}", name);
            let lit_name = LitStr::new(s.as_str(), name.span());
            for variant in e.variants.iter() {
                let vname = format_ident!("_destruct_enum_{}_variant_{}", name, variant.ident);
                let meta_name =
                    format_ident!("_destruct_enum_{}_variant_{}_meta", name, variant.ident);
                let vfields = variant.fields.clone();
                let (field_type, fields) = convert_fields(&variant.fields);
                let struct_is_named = field_type == FieldType::Named;
                if struct_is_named {
                    tt.extend(quote! {
                        #[allow(non_camel_case_types)]
                        #[derive(Debug, PartialEq, Eq)]
                        struct #vname #vfields
                    });
                } else {
                    tt.extend(quote! {
                        #[allow(non_camel_case_types)]
                        #[derive(Debug, PartialEq, Eq)]
                        struct #vname #vfields;
                    });
                }
                let s = format!("{}", name);
                let lit_name = LitStr::new(s.as_str(), name.span());
                let s = format!("{}", vname);
                let lit_vname = LitStr::new(s.as_str(), name.span());
                tt.extend(quote! {
                    #[allow(non_camel_case_types)]
                    struct #meta_name;
                    impl destruct_lib::DestructMetadata for #meta_name {
                        fn struct_name() -> &'static str {
                            #lit_vname
                        }
                        fn named_fields() -> bool {
                            #struct_is_named
                        }
                    }
                    impl destruct_lib::DestructEnumMetadata for #meta_name {
                        fn enum_name() -> &'static str {
                            #lit_name
                        }
                    }
                    impl destruct_lib::DestructEnumVariantMetadata for #meta_name {
                        fn variant_name() -> &'static str {
                            #lit_vname
                        }
                    }
                });
                let s = format!("{}::{}", name, vname);
                let lit_name = LitStr::new(s.as_str(), name.span());
                tt.extend(derive_struct(vname, lit_name, struct_is_named, fields));
            }
            let destruct_enum_meta_name = format_ident!("_destruct_enum_{}_meta", name);
            let destruct_enum_type = get_destruct_enum_type(&name, &mut e.variants.iter());
            let destruct_enum_from = get_destruct_enum_from(&name, &mut e.variants.iter());
            let destruct_enum_into = get_destruct_enum_into(&name, &mut e.variants.iter());
            quote! {
                #tt

                impl From<#name> for destruct_lib::DestructEnumBegin<#destruct_enum_type, #destruct_enum_meta_name> {
                    fn from(t: #name) -> Self {
                        destruct_lib::DestructEnumBegin::new(match t {#destruct_enum_from})
                    }
                }

                #[allow(non_camel_case_types)]
                #[derive(Debug, PartialEq, Eq)]
                struct #destruct_enum_meta_name;

                impl destruct_lib::DestructEnumMetadata for #destruct_enum_meta_name {
                    fn enum_name() -> &'static str {
                        #lit_name
                    }
                }

                impl Into<#name> for destruct_lib::DestructEnumBegin<#destruct_enum_type, #destruct_enum_meta_name> {
                    fn into(self) -> #name {
                        match self.variants {
                            #destruct_enum_into
                        }
                    }
                }

                impl destruct_lib::Destruct for #name {
                    type DestructType = destruct_lib::DestructEnumBegin<#destruct_enum_type, #destruct_enum_meta_name>;

                    fn destruct(self) -> Self::DestructType {
                        self.into()
                    }

                    fn construct(d: Self::DestructType) -> Self {
                        d.into()
                    }
                }
            }
        }
        _ => panic!("derive Destruct supports only structs"),
    };
    proc_macro::TokenStream::from(result)
}

fn derive_struct(
    name: Ident,
    lit_name: LitStr,
    struct_is_named: bool,
    fields: Vec<FieldOrdered>,
) -> TokenStream2 {
    let destruct_type = get_destruct_type(&name, &mut fields.iter());
    let destruct_from = get_destruct_from(&mut fields.iter());
    let self_name = format_ident!("self");
    let destruct_into = get_destruct_into_fields(&self_name, struct_is_named, &mut fields.iter());
    let destruct_field_meta = get_destruct_field_meta(&name, struct_is_named, &mut fields.iter());

    let destruct_meta_name = format_ident!("_destruct_{}_meta", name);

    // Return the generated impl
    let output = quote! {
        impl From<#name> for destruct_lib::DestructBegin<#destruct_type, #destruct_meta_name> {
            fn from(t: #name) -> Self {
                destruct_lib::DestructBegin::new(#destruct_from)
            }
        }

        #[allow(non_camel_case_types)]
        #[derive(Debug, PartialEq, Eq)]
        struct #destruct_meta_name;

        impl destruct_lib::DestructMetadata for #destruct_meta_name {
            fn struct_name() -> &'static str {
                #lit_name
            }
            fn named_fields() -> bool {
                #struct_is_named
            }
        }

        #destruct_field_meta

        impl Into<#name> for destruct_lib::DestructBegin<#destruct_type, #destruct_meta_name> {
            fn into(self) -> #name {
                #name #destruct_into
            }
        }

        impl destruct_lib::Destruct for #name {
            type DestructType = destruct_lib::DestructBegin<#destruct_type, #destruct_meta_name>;

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
