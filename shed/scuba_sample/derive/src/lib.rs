/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use darling::FromField;
use itertools::Either;
use itertools::Itertools;
use proc_macro::TokenStream;
use proc_macro2::Span;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::Data;
use syn::DataStruct;
use syn::Fields;
use syn::Lifetime;

#[proc_macro_derive(StructuredSample, attributes(scuba))]
pub fn structured_sample_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    match syn::parse(input) {
        Ok(ast) => match impl_structured_sample(&ast) {
            Ok(v) => v,
            Err(error) => error.write_errors(),
        },
        Err(error) => syn::Error::to_compile_error(&error),
    }
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
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    match syn::parse(input) {
        Ok(ast) => match impl_try_from_sample(&ast) {
            Ok(v) => v,
            Err(error) => error.write_errors(),
        },
        Err(error) => syn::Error::to_compile_error(&error),
    }
    .into()
}

fn impl_structured_sample(ast: &syn::DeriveInput) -> darling::Result<TokenStream2> {
    let name = &ast.ident;
    let mut error_collector = darling::Error::accumulator();

    let (fields, field_parse_errors): (Vec<SampleField>, Vec<darling::Error>) =
        get_fields(ast, true)?;

    for err in field_parse_errors {
        error_collector.push(err);
    }

    let field_name = get_field_names(&fields);
    let field_renames = get_field_renames(&fields);

    // check for duplicate names
    check_unique(&fields, &field_renames, &mut error_collector);
    let ty_gen = get_lifetime(ast);

    error_collector.finish()?;
    let gen = quote! {
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
    };
    Ok(gen)
}

fn impl_try_from_sample(ast: &syn::DeriveInput) -> darling::Result<TokenStream2> {
    let name = &ast.ident;
    let mut error_collector = darling::Error::accumulator();

    let (fields, field_parse_errors): (Vec<SampleField>, Vec<darling::Error>) =
        get_fields(ast, false)?;

    for err in field_parse_errors {
        error_collector.push(err);
    }

    let field_name = get_field_names(&fields);
    let field_renames = get_field_renames(&fields);
    let field_parsers = fields
        .iter()
        .map(|field| field.get_parser())
        .collect::<Vec<_>>();

    // check for duplicate names
    check_unique(&fields, &field_renames, &mut error_collector);

    error_collector.finish()?;
    let gen = quote! {
        impl ::scuba_sample::TryFromSample for #name {}

        impl ::core::convert::TryFrom<::scuba_sample::ScubaSample> for #name {
            type Error = ::scuba_sample::Error;

            fn try_from(mut sample: ::scuba_sample::ScubaSample) -> ::core::result::Result<Self, ::scuba_sample::Error> {
                #(
                    let #field_name = sample
                        .retrieve(#field_renames)
                        .ok_or_else(|| ::scuba_sample::Error::MissingColumn(
                            ::std::format!(
                                "Could not find {} in ScubaSample {:?}",
                                ::core::stringify!(#field_renames),
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
    };
    Ok(gen)
}

fn get_fields(
    ast: &syn::DeriveInput,
    can_skip: bool,
) -> darling::Result<(Vec<SampleField>, Vec<darling::Error>)> {
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        // These branches are like this to make better compiler errors.
        Data::Struct(s) => {
            return Err(
                darling::Error::unsupported_shape("struct without named fields")
                    .with_span(&s.struct_token.span),
            );
        }
        Data::Enum(s) => {
            return Err(darling::Error::unsupported_shape("enum").with_span(&s.enum_token.span));
        }
        Data::Union(s) => {
            return Err(darling::Error::unsupported_shape("union").with_span(&s.union_token.span));
        }
    };

    Ok(fields
        .into_iter()
        .filter_map(|f| {
            let field = SampleField::from_field(f);
            if let Ok(pfield) = &field {
                if can_skip && pfield.skip {
                    return None;
                }
            }
            Some(field)
        })
        .partition_map(|f| match f {
            Ok(v) => Either::Left(v),
            Err(v) => Either::Right(v),
        }))
}

fn get_field_names<'a>(fields: &'a [SampleField]) -> Vec<&'a Option<syn::Ident>> {
    fields.iter().map(|field| &field.ident).collect::<Vec<_>>()
}

fn get_field_renames(fields: &[SampleField]) -> Vec<String> {
    fields
        .iter()
        .map(|field| field.scuba_column_name())
        .collect::<Vec<_>>()
}

fn check_unique(
    fields: &[SampleField],
    field_renames: &[String],
    error_collector: &mut darling::error::Accumulator,
) {
    let unique = field_renames.iter().cloned().unique().collect::<Vec<_>>();
    if unique.len() != field_renames.len() {
        // determine which fields are duplicates.
        let mut fields_dup = Vec::from_iter(fields);
        for uf in unique {
            let index = fields_dup
                .iter()
                .position(|x| *x.scuba_column_name() == *uf)
                .unwrap();
            fields_dup.remove(index);
        }
        for f in fields_dup {
            error_collector.push(
                darling::Error::custom(format!(
                    "duplicate scuba column name: {}",
                    f.scuba_column_name()
                ))
                .with_span(&f.ident.as_ref().unwrap().span()),
            )
        }
    }
}

fn get_lifetime(ast: &syn::DeriveInput) -> TokenStream2 {
    let mut new_gen = ast.generics.clone();
    new_gen
        .lifetimes_mut()
        .for_each(|lf| lf.lifetime = Lifetime::new("'_", Span::call_site()));
    let (_, ty_gen, _) = new_gen.split_for_impl();
    quote! {#ty_gen}
}

#[derive(Debug, Clone, FromField)]
#[darling(attributes(scuba), forward_attrs(allow, doc, cfg))]
struct SampleField {
    ident: Option<syn::Ident>,
    ty: syn::Type,
    name: Option<String>,
    #[darling(default)]
    skip: bool,
    custom_parser: Option<syn::Ident>,
}

impl SampleField {
    fn scuba_column_name(&self) -> String {
        self.name
            .as_ref()
            .unwrap_or(&self.ident.as_ref().unwrap().to_string().replace('"', ""))
            .to_string()
    }

    fn get_parser(&self) -> TokenStream2 {
        let field_name = self.ident.as_ref().unwrap();
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
