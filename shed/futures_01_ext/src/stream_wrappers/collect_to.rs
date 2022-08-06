/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::mem;

use futures::Async;
use futures::Future;
use futures::Poll;
use futures::Stream;

/// Stream returned as a result of calling [crate::StreamExt::collect_to]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct CollectTo<S, C> {
    stream: S,
    collection: C,
}

impl<S: Stream, C> CollectTo<S, C>
where
    C: Default + Extend<S::Item>,
{
    fn finish(&mut self) -> C {
        mem::take(&mut self.collection)
    }

    /// Create a new instance of [CollectTo] wrapping the provided stream
    pub fn new(stream: S) -> CollectTo<S, C> {
        CollectTo {
            stream,
            collection: Default::default(),
        }
    }
}

impl<S, C> Future for CollectTo<S, C>
where
    S: Stream,
    C: Default + Extend<S::Item>,
{
    type Item = C;
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match self.stream.poll() {
                Ok(Async::Ready(Some(v))) => self.collection.extend(Some(v)),
                Ok(Async::Ready(None)) => return Ok(Async::Ready(self.finish())),
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => {
                    self.finish();
                    return Err(e);
                }
            }
        }
    }
}
