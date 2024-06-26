/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::collections::HashSet;
use std::fmt;
use std::fmt::Display;

use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::Error;
use syn::parse::Result;
use syn::parse_macro_input;
use syn::Data;
use syn::DataStruct;
use syn::DeriveInput;
use syn::ExprPath;
use syn::Field;
use syn::Fields;
use syn::Ident;
use syn::Lifetime;
use syn::LitStr;
use syn::Type;

enum Derive {
    StructuredSample,
    TryFromSample,
}

#[proc_macro_derive(StructuredSample, attributes(scuba))]
pub fn structured_sample_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    impl_structured_sample(&ast)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Derives `TryFrom<ScubaSample>` for some struct.
///
/// Example:
/// ```ignore
/// use std::collections::HashMap;
///
/// use scuba_sample::TryFromSample;
///
/// #[derive(TryFromSample)]
/// struct Foo {
///     bar: i32,
///
///     #[scuba(name = "foo", custom_parser = "my_custom_parser")]
///     map: HashMap<String, String>,
/// }
///
/// fn my_custom_parser(data: String) -> Result<HashMap<String, String>, serde_json::Error> {
///     serde_json::from_str(&data)
/// }
/// ```
///
/// Expands to
/// ```ignore
/// struct Foo {
///     bar: i32,
///     #[scuba(name = "foo", custom_parser = "my_custom_parser")]
///     map: HashMap<String, String>,
/// }
///
/// impl ::scuba_sample::TryFromSample for Foo {}
///
/// impl ::core::convert::TryFrom<::scuba_sample::ScubaSample> for Foo {
///     type Error = ::scuba_sample::Error;
///     fn try_from(
///         mut sample: ::scuba_sample::ScubaSample,
///     ) -> ::core::result::Result<Self, ::scuba_sample::Error> {
///         let bar = sample
///             .retrieve("bar")
///             .ok_or_else(|| ::scuba_sample::Error::MissingColumn({
///                 let res = ::alloc::fmt::format(format_args!(
///                     "Could not find {0} in ScubaSample {1:?}",
///                     "\"bar\"", sample
///                 ));
///                 res
///             }))?;
///         let map = sample
///             .retrieve("foo")
///             .ok_or_else(|| ::scuba_sample::Error::MissingColumn({
///                 let res = ::alloc::fmt::format(format_args!(
///                     "Could not find {0} in ScubaSample {1:?}",
///                     "\"foo\"", sample
///                 ));
///                 res
///             }))?;
///         ::core::result::Result::Ok(Foo {
///             bar: <::scuba_sample::ScubaValue as ::core::convert::TryInto<i32>>::try_into(bar)?,
///             map: my_custom_parser(map.try_into()?).map_err(|e| {
///                 ::scuba_sample::Error::CustomParseError({
///                     let res =
///                         ::alloc::fmt::format(format_args!("Error from custom parser: {0:?}", e));
///                     res
///                 })
///             })?,
///         })
///     }
/// }
/// ```
#[proc_macro_derive(TryFromSample, attributes(scuba))]
pub fn try_from_sample_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    impl_try_from_sample(&ast)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn impl_structured_sample(ast: &DeriveInput) -> Result<TokenStream2> {
    let (fields, mut errors) = get_fields(ast, Derive::StructuredSample)?;

    // check for duplicate names
    check_unique(&fields, &mut errors);
    propagate_errors(errors)?;

    let name = &ast.ident;
    let field_name = get_field_names(&fields);
    let field_renames = get_field_renames(&fields);
    let ty_gen = get_lifetime(ast);

    Ok(quote! {
        impl ::scuba_sample::StructuredSample for #name #ty_gen {}

        impl ::core::convert::From<#name #ty_gen> for ::scuba_sample::ScubaSample {
            fn from(thingy: #name #ty_gen) -> Self {
                let mut sample = ::scuba_sample::ScubaSample::new();
                #(
                    sample.add(#field_renames, thingy.#field_name);
                )*
                sample
            }
        }
    })
}

fn impl_try_from_sample(ast: &DeriveInput) -> Result<TokenStream2> {
    let (fields, mut errors) = get_fields(ast, Derive::TryFromSample)?;

    // check for duplicate names
    check_unique(&fields, &mut errors);
    propagate_errors(errors)?;

    let name = &ast.ident;
    let field_name = get_field_names(&fields);
    let field_renames = get_field_renames(&fields);
    let field_parsers = fields
        .iter()
        .map(|field| field.get_parser())
        .collect::<Vec<_>>();

    Ok(quote! {
        impl ::scuba_sample::TryFromSample for #name {}

        impl ::core::convert::TryFrom<::scuba_sample::ScubaSample> for #name {
            type Error = ::scuba_sample::Error;

            fn try_from(mut sample: ::scuba_sample::ScubaSample) -> ::core::result::Result<Self, ::scuba_sample::Error> {
                #(
                    let #field_name = sample
                        .retrieve(#field_renames)
                        .ok_or_else(|| ::scuba_sample::Error::MissingColumn(
                            ::std::format!(
                                "Could not find {:?} in ScubaSample {:?}",
                                #field_renames,
                                sample,
                            ),
                        ))?;
                )*
                ::core::result::Result::Ok(#name {
                    #(
                        #field_name: #field_parsers,
                    )*
                })
            }
        }
    })
}

fn get_fields(ast: &DeriveInput, derive: Derive) -> Result<(Vec<SampleField>, Vec<Error>)> {
    let Data::Struct(DataStruct {
        fields: Fields::Named(ast_fields),
        ..
    }) = &ast.data
    else {
        return Err(Error::new_spanned(
            ast,
            format!("derive({}) requires a struct with named fields", derive),
        ));
    };

    let can_skip = match derive {
        Derive::StructuredSample => true,
        Derive::TryFromSample => false,
    };

    let mut sample_fields = Vec::new();
    let mut errors = Vec::new();
    for field in &ast_fields.named {
        match SampleField::from_field(field) {
            Ok(sample_field) if can_skip && sample_field.skip => {}
            Ok(sample_field) => sample_fields.push(sample_field),
            Err(error) => errors.push(error),
        }
    }
    Ok((sample_fields, errors))
}

fn get_field_names(fields: &[SampleField]) -> Vec<&Ident> {
    fields.iter().map(|field| &field.ident).collect()
}

fn get_field_renames(fields: &[SampleField]) -> Vec<String> {
    fields
        .iter()
        .map(|field| field.scuba_column_name())
        .collect()
}

fn check_unique(fields: &[SampleField], errors: &mut Vec<Error>) {
    let mut unique = HashSet::new();
    for field in fields {
        let rename = field.scuba_column_name();
        if !unique.insert(rename.clone()) {
            errors.push(Error::new_spanned(
                &field.ident,
                format!("duplicate scuba column name: {}", rename),
            ));
        }
    }
}

fn get_lifetime(ast: &DeriveInput) -> TokenStream2 {
    let mut new_gen = ast.generics.clone();
    new_gen
        .lifetimes_mut()
        .for_each(|lf| lf.lifetime = Lifetime::new("'_", Span::call_site()));
    let (_, ty_gen, _) = new_gen.split_for_impl();
    quote! {#ty_gen}
}

#[derive(Debug, Clone)]
struct SampleField {
    ident: Ident,
    ty: Type,
    rename: Option<LitStr>,
    skip: bool,
    custom_parser: Option<ExprPath>,
}

impl SampleField {
    fn from_field(field: &Field) -> Result<Self> {
        let mut rename = None;
        let mut skip = false;
        let mut custom_parser = None;
        for attr in &field.attrs {
            // Parse #[scuba(...)]
            if attr.path().is_ident("scuba") {
                attr.parse_nested_meta(|meta| {
                    if meta.path.is_ident("name") {
                        // #[scuba(name = "...")]
                        rename = Some(meta.value()?.parse()?);
                        Ok(())
                    } else if meta.path.is_ident("skip") {
                        // #[scuba(skip)]
                        skip = true;
                        Ok(())
                    } else if meta.path.is_ident("custom_parser") {
                        // #[scuba(custom_parser = "path::to")]
                        let lit: LitStr = meta.value()?.parse()?;
                        custom_parser = Some(lit.parse()?);
                        Ok(())
                    } else {
                        Err(meta.error("unrecognized scuba attribute"))
                    }
                })?;
            }
        }

        Ok(SampleField {
            ident: field.ident.as_ref().unwrap().clone(),
            ty: field.ty.clone(),
            rename,
            skip,
            custom_parser,
        })
    }

    fn scuba_column_name(&self) -> String {
        match &self.rename {
            Some(rename) => rename.value(),
            None => self.ident.to_string(),
        }
    }

    fn get_parser(&self) -> TokenStream2 {
        let field_name = &self.ident;
        let ty = &self.ty;
        match self.custom_parser {
            Some(ref parser) => quote! {
                #parser(#field_name.try_into()?).map_err(|e| ::scuba_sample::Error::CustomParseError(::std::format!("Error from custom parser: {:?}", e)))?
            },
            None => quote! {
                <::scuba_sample::ScubaValue as ::core::convert::TryInto<#ty>>::try_into(#field_name)?
            },
        }
    }
}

impl Display for Derive {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(match self {
            Derive::StructuredSample => "StructuredSample",
            Derive::TryFromSample => "TryFromSample",
        })
    }
}

fn propagate_errors(errors: Vec<Error>) -> Result<()> {
    let mut iter = errors.into_iter();
    let Some(mut combined) = iter.next() else {
        return Ok(());
    };
    combined.extend(iter);
    Err(combined)
}
