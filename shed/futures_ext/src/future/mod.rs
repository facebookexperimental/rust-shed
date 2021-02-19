/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Module extending functionality of [`futures::future`] module

mod conservative_receiver;
mod try_shared;

use anyhow::Error;
use futures::{future::TryFuture, Future};
use std::time::Duration;
use tokio::time::Timeout;

pub use shared_error::anyhow::SharedError;

pub use self::conservative_receiver::ConservativeReceiver;
pub use self::try_shared::TryShared;

/// A trait implemented by default for all Futures which extends the standard
/// functionality.
pub trait FbFutureExt: Future {
    /// Create a cloneable handle to this future where all handles will resolve
    /// to the same result.
    ///
    /// Similar to [futures::future::Shared], but instead works on Futures
    /// returning Result where Err is [anyhow::Error].
    /// This is achieved by storing [anyhow::Error] in [std::sync::Arc].
    fn try_shared(self) -> TryShared<Self>
    where
        Self: TryFuture<Error = Error> + Sized,
        <Self as TryFuture>::Ok: Clone,
    {
        self::try_shared::try_shared(self)
    }

    /// Construct a new [tokio::time::Timeout].
    fn timeout(self, timeout: Duration) -> Timeout<Self>
    where
        Self: Sized,
    {
        tokio::time::timeout(timeout, self)
    }
}

impl<T> FbFutureExt for T where T: Future + ?Sized {}
