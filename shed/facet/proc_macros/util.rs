/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use proc_macro2::TokenStream;
use quote::quote;
use syn::Token;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum Asyncness {
    /// Method is synchronous
    Synchronous,

    /// Method is asynchronous
    Asynchronous,
}

impl From<Option<Token![async]>> for Asyncness {
    fn from(asy: Option<Token![async]>) -> Asyncness {
        match asy {
            Some(_) => Asyncness::Asynchronous,
            None => Asyncness::Synchronous,
        }
    }
}

impl Asyncness {
    pub(crate) fn any<'a>(iter: impl IntoIterator<Item = &'a Asyncness>) -> Asyncness {
        if iter.into_iter().any(Asyncness::is_async) {
            Asyncness::Asynchronous
        } else {
            Asyncness::Synchronous
        }
    }

    pub(crate) fn is_async(&self) -> bool {
        matches!(self, Asyncness::Asynchronous)
    }

    pub(crate) fn maybe(&self, if_async: TokenStream) -> TokenStream {
        match self {
            Asyncness::Asynchronous => if_async,
            Asyncness::Synchronous => quote!(),
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum Fallibility {
    /// Method returns the Facet infallibly
    Infallible,

    /// Method returns `Result<Facet, _>`
    Fallible,
}

impl Fallibility {
    pub(crate) fn maybe(&self, if_fallible: TokenStream) -> TokenStream {
        match self {
            Fallibility::Fallible => if_fallible,
            Fallibility::Infallible => quote!(),
        }
    }
}
