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

/// A future which collects all of the values of a stream into a vector.
///
/// This also returns the original stream.
#[derive(Debug)]
#[must_use = "streams do nothing unless polled"]
pub struct CollectNoConsume<S>
where
    S: Stream,
{
    stream: Option<S>,
    items: Vec<S::Item>,
}

pub fn new<S>(s: S) -> CollectNoConsume<S>
where
    S: Stream,
{
    CollectNoConsume {
        stream: Some(s),
        items: Vec::new(),
    }
}

impl<S: Stream> CollectNoConsume<S> {
    fn finish(&mut self) -> (Vec<S::Item>, S) {
        (
            mem::take(&mut self.items),
            self.stream.take().expect("finish called after completion"),
        )
    }
}

impl<S> Future for CollectNoConsume<S>
where
    S: Stream,
{
    type Item = (Vec<S::Item>, S);
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Self::Item, S::Error> {
        loop {
            match self
                .stream
                .as_mut()
                .expect("CollectNoConsume future polled after completion")
                .poll()
            {
                Ok(Async::Ready(Some(e))) => self.items.push(e),
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
