mod maps;
use crate::maps::map_struct;
use quote::quote;
use std::error::Error;
use std::iter::Iterator;
use syn::{
    parse_macro_input, Attribute, Data, DataEnum, DataStruct, DeriveInput, Field, Fields,
    FieldsNamed, FieldsUnnamed, Generics, Ident, Meta, NestedMeta, Type, Variant,
};

fn gen_struct_impl(
    id: Ident,
    generics: Generics,
    nfs: FieldsNamed,
) -> Result<proc_macro2::TokenStream, Box<dyn Error>> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let struct_id_string: String = format!("{}", id.to_string());

    let schema = map_struct(
        &struct_id_string,
        None,
        nfs.iter()
            .filter_map(
                |Field {
                     attrs,
                     ident: Some(id),
                     ty,
                     ..
                 }| if !marked_skip {Some((&id, &ty))} else { None }
            )
            .collect::<Vec<(&Ident, &Type)>>(),
    );

    Ok(quote!(
        impl#impl_generics avro_rs::schema_gen::Schematized for #id #ty_generics #where_clause {
            // Hello world! #impl_generics
            fn schematize(parent_namespace: Option<std::string::String>) -> avro_rs::schema::Schema {
                let id_string = std::string::String::from(#id_string);
                let this_namespace = parent_namespace.as_ref().map_or(id_string.clone(), |ns| std::format!("{}.{}", ns, &id_string));
                #schema
            }
        }
    ))
}

fn gen_enum_impl(
    enum_id: Ident,
    generics: Generics,
    variants: Vec<Variant>,
) -> Result<proc_macro2::TokenStream, Box<dyn Error>> {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let enum_id_string: String = format!("{}", enum_id.to_string());

    map_enum()

    Ok(quote!())
}

#[proc_macro_derive(Schematize)]
pub fn derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let DeriveInput {
        ident: id,
        data,
        generics,
        ..
    } = parse_macro_input!(input as DeriveInput);
    let r = match data {
        Data::Struct(DataStruct {
            fields: Fields::Named(nfs),
            ..
        }) => gen_struct_impl(id, generics, nfs).unwrap(),
        Data::Enum(DataEnum { variants, .. }) => gen_enum_impl(
            id,
            generics,
            variants.iter().map(|v| v.clone()).collect::<Vec<Variant>>(),
        )
        .unwrap(),
        _ => panic!("Can only handle structs with named fields, and enums."),
    };

    proc_macro::TokenStream::from(r)
}
