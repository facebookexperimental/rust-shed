/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

#![cfg_attr(fbcode, feature(backtrace))]
#![deny(warnings, missing_docs, clippy::all, intra_doc_link_resolution_failure)]

//! Crate extending functionality of [`failure`] and [`anyhow`] crates

use std::error::Error as StdError;
use std::fmt::{self, Debug, Display};

// FIXME: cargo_from_buck's behavior on renamed dependencies does not match tp2's
#[cfg(fbcode)]
pub use failure;
#[cfg(not(fbcode))]
pub use failure_deprecated as failure;

mod slogkv;
pub use crate::slogkv::{cause_workaround as cause, SlogKVError, SlogKVErrorKey};

mod convert;
pub use self::convert::convert;

pub mod chain;

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

    pub use crate::chain::{self, Chain, ChainExt};
    pub use crate::{
        FutureFailureErrorExt, FutureFailureExt, StreamFailureErrorExt, StreamFailureExt,
    };
    pub use anyhow::{Context, Error, Result};
}

pub use anyhow::{bail, ensure, format_err, Error, Result};

// Temporary immitation of failure's API to ease migration.
#[doc(hidden)]
pub use anyhow::Context as ResultExt;

// Deprecated.
#[doc(hidden)]
pub use failure::Fail;

#[macro_use]
mod macros;
mod context_futures;
mod context_streams;
pub use crate::context_futures::{FutureFailureErrorExt, FutureFailureExt};
pub use crate::context_streams::{StreamFailureErrorExt, StreamFailureExt};

/// Shallow wrapper struct around ['anyhow::Error`] with ['std::fmt::Display`]
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
    #[cfg(fbcode)]
    fn backtrace(&self) -> Option<&std::backtrace::Backtrace> {
        Some(self.0.backtrace())
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
