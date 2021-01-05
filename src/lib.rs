mod error;
mod maps;

use crate::maps::{extract_named_fields, map_enum, map_struct, map_union, marked_skip};
use quote::quote;
use std::error::Error as ErrorTrait;
use std::iter::Iterator;
use syn::{
    parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, Fields, FieldsNamed, Ident, Variant,
};

fn gen_enum_impl(
    enum_id: &Ident,
    variants: Vec<Variant>,
) -> Result<proc_macro2::TokenStream, Box<dyn ErrorTrait>> {
    let enum_id_string = enum_id.to_string();
    if variants
        .iter()
        .filter_map(|Variant { attrs, fields, .. }| {
            if !marked_skip(attrs) {
                Some(fields)
            } else {
                None
            }
        })
        .all(|f| match f {
            Fields::Unit => true,
            _ => false,
        })
    {
        map_enum(
            None,
            &enum_id,
            variants
                .iter()
                .map(|Variant { ident, .. }| ident.to_string())
                .collect::<Vec<String>>(),
        )
    } else {
        map_union(&enum_id_string, &variants)
    }
}

#[proc_macro_derive(Schematize)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DeriveInput {
        ident: id,
        data,
        generics,
        ..
    } = parse_macro_input!(input as DeriveInput);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let body = match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) => map_struct(&id, extract_named_fields(&named).unwrap()).unwrap(),
        Data::Enum(DataEnum { variants, .. }) => gen_enum_impl(
            &id,
            variants.iter().map(|v| v.clone()).collect::<Vec<Variant>>(),
        )
        .unwrap(),
        _ => panic!("Can only handle structs with named fields, and enums."),
    };
    let id_string = id.to_string();
    quote!(
        impl#impl_generics avro_rs::schema_gen::Schematized for #id #ty_generics #where_clause {
            // Hello world! #impl_generics
            fn schematize(parent_namespace: Option<std::string::String>) -> avro_rs::schema::Schema {
                let id_string = std::string::String::from(#id_string);
                let this_namespace = parent_namespace.as_ref().map_or(id_string.clone(), |ns| std::format!("{}.{}", ns, &id_string));
                #body
            }
        }
    ).into()
}
