use proc_macro2::TokenStream;
use quote::quote;
use std::error::Error;
use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use syn::{GenericArgument, PathArguments, PathSegment};

pub(crate) fn map_ty(namespace: &str, ty: &syn::Type) -> Result<TokenStream, Box<dyn Error>> {
    match ty {
        syn::Type::Path(tp) => map_path(namespace, tp),
        syn::Type::Array(ta) => {
            let inner = map_ty(namespace, &ta.elem)?;
            Ok(quote!(avro_rs::schema::Schema::Array(Box::new(#inner))))
        }
        _ => panic!("Schematize: cannot handle non-Path or Array syn::Type."),
    }
}

pub(crate) fn map_id(namespace: &str, id: &syn::Ident) -> Result<TokenStream, Box<dyn Error>> {
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

#[derive(Debug)]
enum SchematizeError {
    NonTypeGenericArgument(TokenStream),
    MultipleArgs(TokenStream),
    MissingAngleBrackets(TokenStream),
    MissingArgs(TokenStream),
    EmptyPath(TokenStream),
}

impl Display for SchematizeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self, f)
    }
}

impl Error for SchematizeError {}

pub(crate) fn map_box(namespace: &str, seg: &PathSegment) -> Result<TokenStream, Box<dyn Error>> {
    match &seg.arguments {
        PathArguments::AngleBracketed(angle_args) => {
            let args = &angle_args.args;
            match args.len() {
                0 => Err(Box::new(SchematizeError::MissingArgs(quote!(#seg)))),
                1 => match args.first().unwrap() {
                    GenericArgument::Type(t) => map_ty(&namespace, t),
                    _ => Err(Box::new(SchematizeError::NonTypeGenericArgument(
                        quote!(#seg),
                    ))),
                },
                _ => Err(Box::new(SchematizeError::MultipleArgs(quote!(#seg)))),
            }
        }
        _ => Err(Box::new(SchematizeError::MissingAngleBrackets(
            quote!(#seg),
        ))),
    }
}

fn map_option(namespace: &str, seg: &PathSegment) -> Result<TokenStream, Box<dyn Error>> {
    match &seg.arguments {
        syn::PathArguments::AngleBracketed(angle_args) => {
            let args = &angle_args.args;
            match args.len() {
                0 => Err(Box::new(SchematizeError::MissingArgs(quote!(#seg)))),
                1 => match args.first().unwrap() {
                    syn::GenericArgument::Type(t) => {
                        let _inner = map_ty(&namespace, t);
                        unimplemented!()
                    }
                    _ => Err(Box::new(SchematizeError::NonTypeGenericArgument(
                        quote!(#seg),
                    ))),
                },
                _ => Err(Box::new(SchematizeError::MultipleArgs(quote!(#seg)))),
            }
        }
        _ => Err(Box::new(SchematizeError::MissingAngleBrackets(
            quote!(#seg),
        ))),
    }
}

fn map_vec(namespace: &str, seg: &PathSegment) -> Result<TokenStream, Box<dyn Error>> {
    match &seg.arguments {
        syn::PathArguments::AngleBracketed(angle_args) => {
            let args = &angle_args.args;
            match args.len() {
                0 => Err(Box::new(SchematizeError::MissingArgs(quote!(#seg)))),
                1 => match args.first().unwrap() {
                    syn::GenericArgument::Type(t) => {
                        let inner = map_ty(namespace, t)?;
                        Ok(quote!(avro_rs::schema::Schema::Array(Box::new(#inner))))
                    }
                    _ => Err(Box::new(SchematizeError::NonTypeGenericArgument(
                        quote!(#seg),
                    ))),
                },
                _ => Err(Box::new(SchematizeError::MultipleArgs(quote!(#seg)))),
            }
        }
        _ => Err(Box::new(SchematizeError::MissingAngleBrackets(
            quote!(#seg),
        ))),
    }
}

pub(crate) fn map_segs(namespace: &str, seg: &PathSegment) -> Result<TokenStream, Box<dyn Error>> {
    let seg_id_string = seg.ident.to_string();
    match seg_id_string.as_str() {
        "Box" => map_box(&namespace, seg),
        "Option" => map_option(&namespace, seg),
        "Vec" => map_vec(&namespace, seg),
        _ => match &seg.arguments {
            PathArguments::AngleBracketed(angle_args) => {
                let args = &angle_args.args;
                match args.len() {
                    1 => match args.first().unwrap() {
                        // Type::Path(TypePath { path, .. })
                        syn::GenericArgument::Type(t) => {
                            let seg_id = &seg.ident;
                            Ok(quote!(#seg_id::<#t>::schematize(Some(String::from(#namespace)))))
                        }
                        _ => Err(Box::new(SchematizeError::NonTypeGenericArgument(
                            quote!(#seg),
                        ))),
                    },
                    _ => Err(Box::new(SchematizeError::MultipleArgs(quote!(#seg)))),
                }
            }
            _ => Err(Box::new(SchematizeError::MissingAngleBrackets(
                quote!(#seg),
            ))),
        },
    }
}

fn map_path(namespace: &str, tp: &syn::TypePath) -> Result<TokenStream, Box<dyn Error>> {
    if tp.path.segments.is_empty() {
        Err(Box::new(SchematizeError::EmptyPath(quote!(#tp))))
    } else if let Some(id) = tp.path.get_ident() {
        map_id(namespace, id)
    } else if tp.path.segments.len() == 1 {
        map_segs(namespace, tp.path.segments.first().unwrap())
    } else {
        let ids = tp
            .path
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
