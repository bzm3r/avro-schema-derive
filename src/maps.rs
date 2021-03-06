use crate::error::Error;
use proc_macro2::TokenStream;
use quote::quote;
use std::error::Error as ErrorTrait;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::{
    AngleBracketedGenericArguments, Attribute, Field, Fields, FieldsNamed, FieldsUnnamed,
    GenericArgument, Ident, Meta, NestedMeta, Path, PathArguments, PathSegment, Type, TypeArray,
    TypePath, TypeTuple, Variant,
};

pub(crate) fn map_ty(namespace: &str, ty: &syn::Type) -> Result<TokenStream, Box<dyn ErrorTrait>> {
    match ty {
        syn::Type::Path(TypePath { path, .. }) => map_path(namespace, path),
        syn::Type::Array(TypeArray { elem, .. }) => {
            let inner = map_ty(namespace, elem)?;
            Ok(quote!(avro_rs::schema::Schema::Array(Box::new(#inner))))
        }
        syn::Type::Tuple(TypeTuple { elems, .. }) => Ok(map_tuple(
            namespace,
            None,
            elems.iter().collect::<Vec<&Type>>(),
        )?),
        _ => {
            panic!(
                "Schematize: cannot handle non-Path or Array syn::Type. Received: {:?}",
                ty
            );
        }
    }
}

pub(crate) fn map_id(namespace: &str, id: &syn::Ident) -> Result<TokenStream, Box<dyn ErrorTrait>> {
    let id_string = id.to_string();
    match id_string.as_str() {
        "bool" | "i32" | "u32" | "i64" | "f32" | "f64" => Ok(
            quote!(#id::schematize(std::option::Option::Some(std::string::String::from(#namespace)))),
        ),
        "String" => Ok(
            quote!(std::string::String::schematize(std::option::Option::Some(std::string::String::from(#namespace)))),
        ),
        _ => Ok(
            quote!(#id::schematize(std::option::Option::Some(std::format!("{}.{}", this_namespace, #namespace)))),
        ),
    }
}

// pub(crate) fn map_field_id(field_info: Option<&str>, id: &syn::Ident) -> TokenStream {
//     let id_string = id.to_string();
//     match id_string.as_str() {
//         "bool" | "i32" | "u32" | "i64" | "f32" | "f64" => quote!(#id::schematize(None)),
//         "String" => quote!(std::string::String::schematize(None)),
//         _ => {
//             let ns = quote!(namespace.clone().unwrap());
//             let field_name = if let Some(f) = field_name {
//                 quote!(std::String::from(#field_info))
//             } else {
//                 quote!(std::String::new())
//             };
//             quote!(#id::schematize(std::option::Option::Some(std::string::String::from(#namespace))))
//         }
//     }
// }

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self, f)
    }
}

pub(crate) fn map_box(
    namespace: &str,
    seg: &PathSegment,
) -> Result<TokenStream, Box<dyn ErrorTrait>> {
    match &seg.arguments {
        PathArguments::AngleBracketed(angle_args) => {
            let args = &angle_args.args;
            match args.len() {
                0 => Err(Box::new(Error::MissingArgs(quote!(#seg)))),
                1 => match args.first().unwrap() {
                    GenericArgument::Type(t) => map_ty(&namespace, t),
                    _ => Err(Box::new(Error::NonTypeGenericArgument(quote!(#seg)))),
                },
                _ => Err(Box::new(Error::MultipleArgs(quote!(#seg)))),
            }
        }
        _ => Err(Box::new(Error::MissingAngleBrackets(quote!(#seg)))),
    }
}

fn map_option(namespace: &str, seg: &PathSegment) -> Result<TokenStream, Box<dyn ErrorTrait>> {
    match &seg.arguments {
        syn::PathArguments::AngleBracketed(angle_args) => {
            let args = &angle_args.args;
            match args.len() {
                0 => Err(Box::new(Error::MissingArgs(quote!(#seg)))),
                1 => match args.first().unwrap() {
                    syn::GenericArgument::Type(t) => {
                        let _inner = map_ty(&namespace, t);
                        unimplemented!()
                    }
                    _ => Err(Box::new(Error::NonTypeGenericArgument(quote!(#seg)))),
                },
                _ => Err(Box::new(Error::MultipleArgs(quote!(#seg)))),
            }
        }
        _ => Err(Box::new(Error::MissingAngleBrackets(quote!(#seg)))),
    }
}

fn map_vec(namespace: &str, seg: &PathSegment) -> Result<TokenStream, Box<dyn ErrorTrait>> {
    match &seg.arguments {
        syn::PathArguments::AngleBracketed(angle_args) => {
            let args = &angle_args.args;
            match args.len() {
                0 => Err(Box::new(Error::MissingArgs(quote!(#seg)))),
                1 => match args.first().unwrap() {
                    syn::GenericArgument::Type(t) => {
                        let inner = map_ty(namespace, t)?;
                        Ok(quote!(avro_rs::schema::Schema::Array(Box::new(#inner))))
                    }
                    _ => Err(Box::new(Error::NonTypeGenericArgument(quote!(#seg)))),
                },
                _ => Err(Box::new(Error::MultipleArgs(quote!(#seg)))),
            }
        }
        _ => Err(Box::new(Error::MissingAngleBrackets(quote!(#seg)))),
    }
}

pub(crate) fn map_segs(
    namespace: &str,
    seg: &PathSegment,
) -> Result<TokenStream, Box<dyn ErrorTrait>> {
    let seg_id_string = seg.ident.to_string();
    match seg_id_string.as_str() {
        "Box" => map_box(&namespace, seg),
        "Option" => map_option(&namespace, seg),
        "Vec" => map_vec(&namespace, seg),
        _ => match &seg.arguments {
            PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) => {
                match args.len() {
                    1 => match args.first().unwrap() {
                        // Type::Path(TypePath { path, .. })
                        syn::GenericArgument::Type(t) => {
                            let seg_id = &seg.ident;
                            Ok(quote!(#seg_id::<#t>::schematize(Some(String::from(#namespace)))))
                        }
                        _ => Err(Box::new(Error::NonTypeGenericArgument(quote!(#seg)))),
                    },
                    _ => Err(Box::new(Error::MultipleArgs(quote!(#seg)))),
                }
            }
            _ => Err(Box::new(Error::MissingAngleBrackets(quote!(#seg)))),
        },
    }
}

fn map_path(namespace: &str, path: &Path) -> Result<TokenStream, Box<dyn ErrorTrait>> {
    if path.segments.is_empty() {
        Err(Box::new(Error::EmptyPath(quote!(#path))))
    } else if let Some(id) = path.get_ident() {
        map_id(namespace, id)
    } else if path.segments.len() == 1 {
        map_segs(namespace, path.segments.first().unwrap())
    } else {
        let ids = path
            .segments
            .iter()
            .filter_map(|seg| match seg.arguments {
                syn::PathArguments::None => Some(seg.ident.clone()),
                _ => None,
            })
            .collect::<Vec<syn::Ident>>();
        Ok(quote!(#(#ids)::*))
    }
}

pub(crate) fn marked_skip(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| match attr.parse_meta() {
        Ok(meta) => match meta {
            Meta::List(ml) => {
                ml.path.is_ident("serde")
                    && ml.nested.iter().any(|nm| match nm {
                        NestedMeta::Meta(inner) => {
                            inner.path().is_ident("skip")
                                | inner.path().is_ident("skip_serialize")
                                | inner.path().is_ident("skip_deserialize")
                        }
                        _ => false,
                    })
            }
            _ => false,
        },
        _ => false,
    })
}

pub(crate) fn map_tuple(
    namespace: &str,
    variant: Option<&str>,
    tys: Vec<&Type>,
) -> Result<TokenStream, Box<dyn ErrorTrait>> {
    let namespace = if let Some(v) = variant {
        format!("{}.{}", namespace, v)
    } else {
        namespace.to_string()
    };

    let mut schemas: Vec<TokenStream> = Vec::with_capacity(tys.len());
    for ty in tys.iter() {
        schemas.push(map_ty(&namespace, ty)?)
    }
    let positions = (0..tys.len()).collect::<Vec<usize>>();

    Ok(quote!(
    avro_rs::schema::Schema::Record {
        name: avro_rs::schema::Name {
            name: std::format!("Tuple(#(#tys),*)"),
            namespace: Some(std::string::String::from(#namespace)),
            aliases: None,
        },
        doc: None,
        fields: std::vec![#(
            avro_rs::schema::RecordField {
                name: #positions.to_string(),
                doc: None,
                default: None,
                schema: #schemas,
                order: avro_rs::schema::RecordFieldOrder::Ascending,
                position: #positions,
            }
        ),*],
        lookup: {
            let mut r: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            #(r.insert(#positions.to_string(), #positions);)*
            r
        },
    }))
}

pub(crate) fn map_struct(
    id: &Ident,
    fields: Vec<(&Ident, &Type)>,
) -> Result<TokenStream, Box<dyn ErrorTrait>> {
    let struct_id_string = id.to_string();
    let mut fids: Vec<&Ident> = Vec::with_capacity(fields.len());
    let mut fid_strings: Vec<String> = Vec::with_capacity(fields.len());
    let mut fschemas: Vec<proc_macro2::TokenStream> = Vec::with_capacity(fields.len());
    let mut fpositions: Vec<usize> = Vec::with_capacity(fields.len());

    for (pos, (fid, fty)) in fields.iter().enumerate() {
        let fid_str = fid.to_string();
        let schema = map_ty(&struct_id_string, &fty);
        fids.push(fid);
        fschemas.push(schema?);
        fid_strings.push(fid_str);
        fpositions.push(pos);
    }

    Ok(quote!(avro_rs::schema::Schema::Record {
        name: avro_rs::schema::Name {
            name: std::string::String::from(#struct_id_string),
            namespace: Some(this_namespace.clone()),
            aliases: None,
        },
        doc: None,
        fields: vec![#(
            avro_rs::schema::RecordField {
                name: std::string::String::from(#fid_strings),
                doc: None,
                default: None,
                schema: #fschemas,
                order: avro_rs::schema::RecordFieldOrder::Ascending,
                position: #fpositions,
            }
        ),*],
        lookup: {
            let mut r: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            #(r.insert(std::string::String::from(#fid_strings), #fpositions);)*
            r
        },
    }))
}

pub(crate) fn extract_named_fields(
    named: &Punctuated<Field, Comma>,
) -> Result<Vec<(&Ident, &Type)>, Box<dyn ErrorTrait>> {
    Ok(named
        .iter()
        .filter_map(
            |Field {
                 attrs, ident, ty, ..
             }| {
                let id = ident
                    .as_ref()
                    .map_or(Err(Error::NamedFieldMissingIdent), |n| Ok(n))
                    .ok()?;
                if !marked_skip(attrs) {
                    Some((id, ty))
                } else {
                    None
                }
            },
        )
        .collect::<Vec<(&Ident, &Type)>>())
}

pub(crate) fn map_union(
    enum_id_string: &str,
    variants: &Vec<Variant>,
) -> Result<TokenStream, Box<dyn ErrorTrait>> {
    let mut vids: Vec<Ident> = Vec::with_capacity(variants.len());
    let mut vid_strings: Vec<String> = Vec::with_capacity(variants.len());
    let mut vschemas: Vec<proc_macro2::TokenStream> = Vec::with_capacity(variants.len());
    let mut vpositions: Vec<usize> = Vec::with_capacity(variants.len());

    for (pos, variant) in variants.iter().enumerate() {
        let syn::Variant {
            ident,
            attrs,
            fields,
            ..
        } = variant;
        if !marked_skip(&attrs) {
            let variant_id_string = ident.to_string();
            let namespace = format!("{}.{}", enum_id_string, &variant_id_string);
            let schema = match fields {
                Fields::Named(FieldsNamed { named, .. }) => {
                    map_struct(&ident, extract_named_fields(named)?)
                }
                Fields::Unnamed(FieldsUnnamed { unnamed, .. }) => map_tuple(
                    &namespace,
                    Some(&variant_id_string),
                    unnamed
                        .iter()
                        .filter_map(
                            |Field { attrs, ty, .. }| {
                                if !marked_skip(attrs) {
                                    Some(ty)
                                } else {
                                    None
                                }
                            },
                        )
                        .collect::<Vec<&Type>>(),
                ),
                Fields::Unit => Ok(quote!(avro_rs::schema::Schema::RecordField {
                    name: std::string::String::from(#variant_id_string),
                    doc: None,
                    default: None,
                    schema: avro_rs::schema::Schema::Null,
                    order: avro_rs::schema::RecordFieldOrder::Ascending,
                    position: #pos,
                })),
            };
            vids.push(ident.clone());
            vschemas.push(schema?);
            vid_strings.push(variant_id_string);
            vpositions.push(pos);
        }
    }

    Ok(quote!(avro_rs::schema::Schema::Union(
        avro_rs::schema::Schema::UnionSchema::new(std::vec![#(#vschemas)*,])
    )))
}

pub(crate) fn map_enum(
    namespace: Option<&str>,
    id: &Ident,
    variants: Vec<String>,
) -> Result<TokenStream, Box<dyn ErrorTrait>> {
    let id_string = id.to_string();
    let namespace = if let Some(ns) = namespace {
        quote!(std::option::Option::Some(&std::string::String::from(#ns)))
    } else {
        quote!(std::option::Option::None)
    };
    Ok(quote!(avro_rs::schema::Schema::Enum {
        name: Name {
            name: std::string::String::from(#id_string),
            namespace: std::option::Option::Some(std::string::String::from(#namespace)),
            aliases: None,
        },
        docs: None,
        symbols: std::vec![#(std::string::String::from(#variants))*,],
    }))
}
