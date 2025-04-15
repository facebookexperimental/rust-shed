/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Module introduces a proc macro for sql_common::mysql.

use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;
use syn::parse_macro_input;

/// The proc macro allows to derive an implementation of mysql_client::OptionalTryFromRowField
/// trait for the type if that type implements mysql_async::FromValueOpt.
/// This is the one to use if NULL maps to None
#[proc_macro_derive(OptTryFromRowField)]
pub fn derive_opttryfrom_rowfield(input: TokenStream) -> TokenStream {
    let parsed_input = parse_macro_input!(input as DeriveInput);
    let name = parsed_input.ident;

    let expanded = quote! {
        impl mysql::OptionalTryFromRowField for #name {
            fn try_from_opt(field: mysql::RowField) -> Result<Option<Self>, mysql::ValueError> {
                mysql::opt_try_from_rowfield(field)
            }
        }
    };
    expanded.into()
}

/// The proc macro allows to derive an implementation of mysql_client::TryFromRowField
/// trait for the type if that type implements mysql_async::FromValueOpt.
/// This is the one to use if NULL does not map to None
#[proc_macro_derive(TryFromRowField)]
pub fn derive_tryfrom_rowfield(input: TokenStream) -> TokenStream {
    let parsed_input = parse_macro_input!(input as DeriveInput);
    let name = parsed_input.ident;

    let expanded = quote! {
        impl mysql::TryFromRowField for #name {
            fn try_from(field: mysql::RowField) -> Result<Self, mysql::ValueError> {
                mysql::opt_try_from_rowfield(field)
            }
        }
    };
    expanded.into()
}
