/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DataStruct, Fields};

#[proc_macro_derive(StructuredSample)]
pub fn structured_sample_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree
    // that we can manipulate
    match syn::parse(input) {
        Ok(ast) => impl_structured_sample(&ast),
        Err(error) => syn::Error::to_compile_error(&error).into(),
    }
}

fn impl_structured_sample(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let fields = match &ast.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => &fields.named,
        _ => {
            return syn::Error::to_compile_error(&syn::Error::new(
                name.span(),
                "expected a struct with named fields",
            ))
            .into();
        }
    };
    let field_name = fields.iter().map(|field| &field.ident);
    let gen = quote! {
        impl ::scuba_sample::StructuredSample for self::#name {}

        impl From<self::#name> for ::scuba_sample::ScubaSample {
            fn from(thingy: self::#name) -> Self {
                let mut sample = ::scuba_sample::ScubaSample::new();
                #(
                    sample.add(stringify!(#field_name), thingy.#field_name);
                )*
                sample
            }
        }
    };
    gen.into()
}
