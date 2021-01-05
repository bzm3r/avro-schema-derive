use proc_macro2::TokenStream;
use std::error::Error as ErrorTrait;

#[derive(Debug)]
pub enum Error {
    NonTypeGenericArgument(TokenStream),
    MultipleArgs(TokenStream),
    MissingAngleBrackets(TokenStream),
    MissingArgs(TokenStream),
    EmptyPath(TokenStream),
    NamedFieldMissingIdent(TokenStream),
}

impl ErrorTrait for Error {}
