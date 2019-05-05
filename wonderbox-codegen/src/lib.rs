#![feature(proc_macro_diagnostic)]

extern crate proc_macro;

mod spanned;

use crate::spanned::SpannedUnstable;
use proc_macro::{Diagnostic, Level, TokenStream};
use quote::quote;
use syn::{
    parse_macro_input, parse_quote, punctuated::Punctuated, token::Comma, AttributeArgs, FnArg,
    FnDecl, ImplItem, ImplItemMethod, Item, ItemImpl, MethodSig, ReturnType, Type,
};

type Result<T> = std::result::Result<T, Diagnostic>;

#[proc_macro_attribute]
pub fn resolve_dependencies(attr: TokenStream, item: TokenStream) -> TokenStream {
    let item = parse_macro_input!(item as Item);
    let attr = parse_macro_input!(attr as AttributeArgs);

    let result = generate_autoresolvable_impl(&item);

    match result {
        Ok(token_stream) => token_stream,
        Err(diagnostic) => {
            diagnostic.emit();
            TokenStream::from(quote! {
                #item
            })
        }
    }
}

fn generate_autoresolvable_impl(item: &Item) -> Result<TokenStream> {
    let item = parse_item_impl(item);

    validate_item_impl(&item);

    let self_ty = &item.self_ty;

    let constructors = parse_constructors(&item);

    if constructors.len() != 1 {
        let error_message = format!("Expected one constructor, found {}", constructors.len());
        return Err(Diagnostic::spanned(
            item.span_unstable(),
            Level::Error,
            error_message,
        ));
    }

    let constructor = constructors.first().unwrap();

    let constructor_args: Result<Vec<_>> = constructor
        .decl
        .inputs
        .iter()
        .map(|arg| match arg {
            FnArg::SelfRef(_) | FnArg::SelfValue(_) => unreachable!(),
            FnArg::Captured(arg) => Ok(&arg.ty),
            _ => Err(Diagnostic::spanned(
                arg.span_unstable(),
                Level::Error,
                "Only normal, non self type parameters are supported",
            )),
        })
        .collect();

    let resolutions: Punctuated<_, Comma> = constructor_args?
        .into_iter()
        .map(|type_| {
            quote! {
                container.resolve::<#type_>()?
            }
        })
        .collect();

    let (impl_generics, type_generics, where_clause) = item.generics.split_for_impl();
    let ident = &constructor.ident;

    Ok(TokenStream::from(quote! {
        #item

        impl #impl_generics wonderbox::internal::AutoResolvable for #self_ty #type_generics #where_clause {
             fn resolve(container: &wonderbox::Container) -> Option<Self> {
                Some(Self::#ident(#resolutions))
             }
        }
    }))
}

fn parse_item_impl(item: &Item) -> &ItemImpl {
    match item {
        Item::Impl(item_impl) => item_impl,
        _ => panic!("{} needs to be placed over an impl block", ATTRIBUTE_NAME),
    }
}

fn validate_item_impl(item_impl: &ItemImpl) -> Result<()> {
    if item_impl.trait_.is_none() {
        let error_message = format!(
            "{} must be placed over a direct impl, not a trait impl",
            ATTRIBUTE_NAME
        );
        Err(Diagnostic::spanned(
            item_impl.span_unstable(),
            Level::Error,
            error_message,
        ))
    } else {
        Ok(())
    }
}

type FunctionArguments = Punctuated<FnArg, Comma>;

fn parse_constructors(item_impl: &ItemImpl) -> Vec<&MethodSig> {
    item_impl
        .items
        .iter()
        .filter_map(parse_method_signature)
        .filter(|declaration| returns_self(&declaration.decl, &item_impl.self_ty))
        .filter(|inputs| has_no_self_parameter(&inputs.decl))
        .collect()
}

fn parse_method_signature(impl_item: &ImplItem) -> Option<&MethodSig> {
    match impl_item {
        ImplItem::Method(method) => Some(&method.sig),
        _ => None,
    }
}

fn returns_self(function: &FnDecl, explicit_self_type: &Type) -> bool {
    match &function.output {
        ReturnType::Default => false,
        ReturnType::Type(_, return_type) => {
            **return_type == generate_self_type() || **return_type == *explicit_self_type
        }
    }
}

fn has_no_self_parameter(function: &FnDecl) -> bool {
    let first_input = function.inputs.first();
    match first_input {
        Some(first_arg) => match first_arg.value() {
            FnArg::SelfRef(_) | FnArg::SelfValue(_) => false,
            _ => true,
        },
        None => true,
    }
}

fn generate_self_type() -> Type {
    parse_quote! {
        Self
    }
}

const ATTRIBUTE_NAME: &str = "#[resolve_dependencies]";
