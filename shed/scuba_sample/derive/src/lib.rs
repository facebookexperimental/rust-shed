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

fn impl_structured_sample(ast: &syn::DeriveInput) -> darling::Result<TokenStream2> {
    let name = &ast.ident;
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
    let mut error_collector = darling::Error::accumulator();
    let (fields, field_parse_errors): (Vec<SampleField>, Vec<darling::Error>) = fields
        .into_iter()
        .filter_map(|f| {
            let field = SampleField::from_field(f);
            if let Ok(pfield) = &field {
                if pfield.skip {
                    return None;
                }
            }
            Some(field)
        })
        .partition_map(|f| match f {
            Ok(v) => Either::Left(v),
            Err(v) => Either::Right(v),
        });
    for err in field_parse_errors {
        error_collector.push(err);
    }

    let field_name = fields.iter().map(|field| &field.ident).collect::<Vec<_>>();
    let field_renames = fields
        .iter()
        .map(|field| field.scuba_column_name())
        .collect::<Vec<_>>();

    // check for duplicate names
    let unique = field_renames.iter().cloned().unique().collect::<Vec<_>>();
    if unique.len() != field_name.len() {
        // determine which fields are duplicates.
        let mut fields_dup = fields.clone();
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

    error_collector.finish()?;
    let mut new_gen = ast.generics.clone();
    new_gen
        .lifetimes_mut()
        .for_each(|lf| lf.lifetime = Lifetime::new("'_", Span::call_site()));
    let (_, ty_gen, _) = new_gen.split_for_impl();
    let gen = quote! {
        impl ::scuba_sample::StructuredSample for #name #ty_gen {}

        impl From<#name #ty_gen> for ::scuba_sample::ScubaSample {
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

#[derive(Debug, Clone, FromField)]
#[darling(attributes(scuba), forward_attrs(allow, doc, cfg))]
struct SampleField {
    ident: Option<syn::Ident>,
    name: Option<String>,
    #[darling(default)]
    skip: bool,
}

impl SampleField {
    fn scuba_column_name(&self) -> String {
        self.name
            .as_ref()
            .unwrap_or(&self.ident.as_ref().unwrap().to_string().replace('"', ""))
            .to_string()
    }
}
