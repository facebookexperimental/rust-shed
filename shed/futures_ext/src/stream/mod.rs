/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Module extending functionality of [`futures::stream`] module

mod return_remainder;
mod weight_limited_buffered_stream;

use futures::{Future, Stream};

use crate::future::ConservativeReceiver;

pub use self::return_remainder::ReturnRemainder;
pub use self::weight_limited_buffered_stream::{BufferedParams, WeightLimitedBufferedStream};

/// A trait implemented by default for all Streams which extends the standard
/// functionality.
pub trait FbStreamExt: Stream {
    /// Creates a stream wrapper and a future. The future will resolve into the wrapped stream when
    /// the stream wrapper returns None. It uses ConservativeReceiver to ensure that deadlocks are
    /// easily caught when one tries to poll on the receiver before consuming the stream.
    fn return_remainder(self) -> (ReturnRemainder<Self>, ConservativeReceiver<Self>)
    where
        Self: Sized,
    {
        ReturnRemainder::new(self)
    }

    /// Like [futures::stream::StreamExt::buffered] call,
    /// but can also limit number of futures in a buffer by "weight".
    fn buffered_weight_limited<I, Fut>(
        self,
        params: BufferedParams,
    ) -> WeightLimitedBufferedStream<Self, I>
    where
        Self: Sized + Send + 'static,
        Self: Stream<Item = (Fut, u64)>,
        Fut: Future<Output = I>,
    {
        WeightLimitedBufferedStream::new(params, self)
    }
}

impl<T> FbStreamExt for T where T: Stream {}
