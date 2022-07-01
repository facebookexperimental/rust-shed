/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]

//! Procedural macros for the `facet` crate.
//!
//! See the crate-level documentation for the `facet` crate for details.

extern crate proc_macro;

use proc_macro_crate::crate_name;
use proc_macro_crate::FoundCrate;

mod container_impl;
mod facet_impl;
mod factory_impl;
mod util;

fn facet_crate_name() -> String {
    match crate_name("facet") {
        Ok(FoundCrate::Name(n)) => n,
        _ => "facet".to_string(),
    }
}

/// Mark a `struct` as a facet container.  See the crate-level documentation
/// for the `facet` crate for details.
#[proc_macro_attribute]
pub fn container(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    container_impl::container(attr, item)
}

/// Mark a `trait` as a facet.  See the crate-level documentation for the
/// `facet` crate for details.
#[proc_macro_attribute]
pub fn facet(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    facet_impl::facet(attr, item)
}

/// Mark an `impl` block as defining a facet factory.  See the crate-level
/// documentation for the `facet` crate for details.
#[proc_macro_attribute]
pub fn factory(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    factory_impl::factory(attr, item)
}
