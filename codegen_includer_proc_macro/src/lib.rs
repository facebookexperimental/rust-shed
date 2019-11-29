/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

#![deny(warnings)]

extern crate proc_macro;
use proc_macro::TokenStream;
use quote::quote;
use std::env;
use std::path::Path;

#[proc_macro]
pub fn include(_: TokenStream) -> TokenStream {
    let path_to_include = Path::new(&env::var("OUT_DIR").unwrap())
        .join("lib.rs")
        .to_str()
        .unwrap()
        .to_string();

    let result = quote! {
        #[path = #path_to_include]
        #[allow(unused_attributes)]
        mod codegen_included;
        pub use codegen_included::*;
    };
    result.into()
}
