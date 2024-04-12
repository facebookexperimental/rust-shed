/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::fmt;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use futures_util::stream::Fuse;
use futures_util::stream::FuturesOrdered;
use futures_util::Future;
use futures_util::Stream;
use futures_util::StreamExt as _;
use pin_project::pin_project;

use crate::global_weight::GlobalWeight;
use crate::peekable_fused::PeekableFused;

/// Stream for the [`buffered_weighted`](crate::StreamExt::buffered_weighted) method.
#[must_use = "streams do nothing unless polled"]
#[pin_project]
pub struct BufferedWeighted<St>
where
    St: Stream,
    St::Item: WeightedFuture,
{
    #[pin]
    stream: PeekableFused<Fuse<St>>,
    in_progress_queue: FuturesOrdered<FutureWithWeight<<St::Item as WeightedFuture>::Future>>,
    global_weight: GlobalWeight,
}

impl<St> fmt::Debug for BufferedWeighted<St>
where
    St: Stream + fmt::Debug,
    St::Item: WeightedFuture,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BufferedWeighted")
            .field("stream", &self.stream)
            .field("in_progress_queue", &self.in_progress_queue)
            .field("global_weight", &self.global_weight)
            .finish()
    }
}

impl<St> BufferedWeighted<St>
where
    St: Stream,
    St::Item: WeightedFuture,
{
    pub(crate) fn new(stream: St, max_weight: usize) -> Self {
        Self {
            stream: PeekableFused::new(stream.fuse()),
            in_progress_queue: FuturesOrdered::new(),
            global_weight: GlobalWeight::new(max_weight),
        }
    }

    /// Returns the maximum weight of futures allowed to be run by this adaptor.
    pub fn max_weight(&self) -> usize {
        self.global_weight.max()
    }

    /// Returns the currently running weight of futures.
    pub fn current_weight(&self) -> usize {
        self.global_weight.current()
    }

    /// Acquires a reference to the underlying sink or stream that this combinator is
    /// pulling from.
    pub fn get_ref(&self) -> &St {
        self.stream.get_ref().get_ref()
    }

    /// Acquires a mutable reference to the underlying sink or stream that this
    /// combinator is pulling from.
    ///
    /// Note that care must be taken to avoid tampering with the state of the
    /// sink or stream which may otherwise confuse this combinator.
    pub fn get_mut(&mut self) -> &mut St {
        self.stream.get_mut().get_mut()
    }

    /// Acquires a pinned mutable reference to the underlying sink or stream that this
    /// combinator is pulling from.
    ///
    /// Note that care must be taken to avoid tampering with the state of the
    /// sink or stream which may otherwise confuse this combinator.
    pub fn get_pin_mut(self: Pin<&mut Self>) -> core::pin::Pin<&mut St> {
        self.project().stream.get_pin_mut().get_pin_mut()
    }

    /// Consumes this combinator, returning the underlying sink or stream.
    ///
    /// Note that this may discard intermediate state of this combinator, so
    /// care should be taken to avoid losing resources when this is called.
    pub fn into_inner(self) -> St {
        self.stream.into_inner().into_inner()
    }
}

impl<St> Stream for BufferedWeighted<St>
where
    St: Stream,
    St::Item: WeightedFuture,
{
    type Item = <<St::Item as WeightedFuture>::Future as Future>::Output;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        // First up, try to spawn off as many futures as possible by filling up
        // our queue of futures.
        while let Poll::Ready(Some(weighted_future)) = this.stream.as_mut().poll_peek(cx) {
            if !this.global_weight.has_space_for(weighted_future.weight()) {
                // Global limits would be exceeded, break out of the loop. Consider this
                // item next time.
                break;
            }

            let (weight, future) = match this.stream.as_mut().poll_next(cx) {
                Poll::Ready(Some(weighted_future)) => weighted_future.into_components(),
                _ => unreachable!("we just peeked at this item"),
            };
            this.global_weight.add_weight(weight);
            this.in_progress_queue
                .push_back(FutureWithWeight::new(weight, future));
        }

        // Attempt to pull the next value from the in_progress_queue.
        match this.in_progress_queue.poll_next_unpin(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Some((weight, output))) => {
                this.global_weight.sub_weight(weight);
                return Poll::Ready(Some(output));
            }
            Poll::Ready(None) => {}
        }

        // If more values are still coming from the stream, we're not done yet
        if this.stream.is_done() {
            Poll::Ready(None)
        } else {
            Poll::Pending
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let queue_len = self.in_progress_queue.len();
        let (lower, upper) = self.stream.size_hint();
        let lower = lower.saturating_add(queue_len);
        let upper = match upper {
            Some(x) => x.checked_add(queue_len),
            None => None,
        };
        (lower, upper)
    }
}

/// A trait for types which can be converted into a `Future` and a weight.
///
/// Provided in case it's necessary. This trait is only implemented for `(usize, impl Future)`.
pub trait WeightedFuture: private::Sealed {
    /// The associated `Future` type.
    type Future: Future;

    /// The weight of the future.
    fn weight(&self) -> usize;

    /// Turns self into its components.
    fn into_components(self) -> (usize, Self::Future);
}

mod private {
    pub trait Sealed {}
}

impl<Fut> private::Sealed for (usize, Fut) where Fut: Future {}

impl<Fut> WeightedFuture for (usize, Fut)
where
    Fut: Future,
{
    type Future = Fut;

    #[inline]
    fn weight(&self) -> usize {
        self.0
    }

    #[inline]
    fn into_components(self) -> (usize, Self::Future) {
        self
    }
}

#[must_use = "futures do nothing unless polled"]
#[pin_project]
struct FutureWithWeight<Fut> {
    #[pin]
    future: Fut,
    weight: usize,
}

impl<Fut> FutureWithWeight<Fut> {
    pub fn new(weight: usize, future: Fut) -> Self {
        Self { future, weight }
    }
}

impl<Fut> Future for FutureWithWeight<Fut>
where
    Fut: Future,
{
    type Output = (usize, Fut::Output);
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();

        match this.future.poll(cx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(output) => Poll::Ready((*this.weight, output)),
        }
    }
}
