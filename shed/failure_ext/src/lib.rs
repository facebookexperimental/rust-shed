/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![cfg_attr(fbcode_build, feature(backtrace))]
#![cfg_attr(fbcode_build, feature(error_generic_member_access))]
#![cfg_attr(fbcode_build, feature(provide_any))]
#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]

//! Crate extending functionality of the [`anyhow`] crate

use std::error::Error as StdError;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;

use anyhow::Error;

mod slogkv;
pub use crate::slogkv::cause_workaround as cause;
pub use crate::slogkv::SlogKVError;
pub use crate::slogkv::SlogKVErrorKey;
pub use crate::slogkv::SlogKVErrorWithoutBackTrace;

pub mod prelude {
    //! A "prelude" of `failure_ext` crate.
    //!
    //! This prelude is similar to the standard library's prelude in that you'll
    //! almost always want to import its entire contents, but unlike the standard
    //! library's prelude you'll have to do so manually:
    //!
    //! ```
    //! # #![allow(unused)]
    //! use failure_ext::prelude::*;
    //! ```

    pub use crate::FutureErrorContext;
    pub use crate::StreamErrorContext;
}

#[macro_use]
mod macros;
mod context_futures;
mod context_streams;
pub use crate::context_futures::FutureErrorContext;
pub use crate::context_streams::StreamErrorContext;

/// Shallow wrapper struct around [anyhow::Error] with [std::fmt::Display]
/// implementation that shows the entire chain of errors
pub struct DisplayChain<'a>(&'a Error);

impl<'a> From<&'a Error> for DisplayChain<'a> {
    fn from(e: &'a Error) -> Self {
        DisplayChain(e)
    }
}

impl Display for DisplayChain<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let e = self.0;
        writeln!(fmt, "Error: {}", e)?;
        for c in e.chain().skip(1) {
            writeln!(fmt, "Caused by: {}", c)?;
        }
        Ok(())
    }
}

/// Temporary immitation of failure::Compat<T> to ease migration.
pub struct Compat<T>(pub T);

impl StdError for Compat<Error> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.0.source()
    }

    #[cfg(fbcode_build)]
    fn provide<'a>(&'a self, demand: &mut std::any::Demand<'a>) {
        demand.provide_ref::<std::backtrace::Backtrace>(self.0.backtrace());
    }
}

impl Display for Compat<Error> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

impl Debug for Compat<Error> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        Debug::fmt(&self.0, formatter)
    }
}
