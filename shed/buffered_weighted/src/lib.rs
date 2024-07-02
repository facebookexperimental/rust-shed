/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![warn(missing_docs)]

//! `buffered_weighted` provides ways to run several futures:
//!
//! * concurrently
//! * in the order they're spawned AND enqueued
//! * with global limits
//!
//! It essentially provides the buffered and ordered semantics of the standard [`buffered`](https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.buffered) combinator in
//! futures crate but with an important distinction. The futures crate buffered variant treats every future the
//! same (in terms of cost), but this crate allows each future to have a different weight. The constraint on the
//! concurrency of polled futures is not the overall count of the futures, but the combined weight of the futures
//!
//! * Futures are started in the order the stream returns them in.
//! * Once started, futures are polled simultaneously, and completed future outputs are returned
//!   in the SAME enqueue order.
//!
//! # When would you use it?
//! When your workload is highly I/O based and you want to leverage as much concurrency as possible. In such cases,
//! the limiting factor for your concurrency is the amount of memory available to run the workload. The items in your
//! stream are uneven ,i.e some correspond to a network fetch of few KBs while others end up fetching 100s of MBs.
//! Instead of using a count based constaint on the concurrent polling of the futures, you can use this crate to apply
//! a weight based constraint where the weight of each future can be tied to size of the fetch so at any given time
//! the system will poll no more than `max_weight` bytes worth of futures thus helping maintain consistent memory use.
//!
//! # What if I don't want ordered semantics?
//! Then you should use the [`future_queue`](https://docs.rs/future-queue/latest/future_queue/index.html) crate which
//! provides the same behavior but with [`buffer_unordered`](https://docs.rs/futures/latest/futures/stream/trait.StreamExt.html#method.buffer_unordered).
//! This crate is completely based on the `future_queue` crate and is created specifically to support the `buffered` usecase.
//!
//! # About this crate
//!
//! This crate provides the following adaptor on streams.
//!
//! ## 1. The `buffered_weighted` adaptor
//!
//! The [`buffered_weighted`](StreamExt::buffered_weighted) adaptor can run several futures simultaneously,
//! limiting the concurrency to a maximum *weight*.
//!
//! Rather than taking a stream of futures, this adaptor takes a stream of `(usize, future)` pairs,
//! where the `usize` indicates the weight of each future. This adaptor will schedule and buffer
//! futures to be run until queueing the next future will exceed the maximum weight.
//!
//! * The maximum weight is never exceeded while futures are being run.
//! * If the weight of an individual future is greater than the maximum weight, its weight will be
//!   set to the maximum weight.
//!
//! Once all possible futures are scheduled, this adaptor will wait until some of the currently
//! executing futures complete, and the current weight of running futures drops below the maximum
//! weight, before scheduling new futures.
//!
//! The weight of a future can be zero, in which case it doesn't count towards the maximum weight.
//!
//! If all weights are 1, then `buffered_weighted` is exactly the same as `buffered`.
//!
//! ### Examples
//!
//! ```rust
//! # futures::executor::block_on(async {
//! use buffered_weighted::StreamExt as _;
//! use futures::channel::oneshot;
//! use futures::stream;
//! use futures::StreamExt as _;
//!
//! let (send_one, recv_one) = oneshot::channel();
//! let (send_two, recv_two) = oneshot::channel();
//!
//! let stream_of_futures = stream::iter(vec![(1, recv_one), (2, recv_two)]);
//! let mut buffered = stream_of_futures.buffered_weighted(5);
//!
//! // Send the second one before the first one. The result should still appear
//! // in the order they were enqueued in the stream, i.e. "world" then "hello".
//! send_two.send("hello")?;
//! send_one.send("world")?;
//! assert_eq!(buffered.next().await, Some(Ok("world")));
//!
//! assert_eq!(buffered.next().await, Some(Ok("hello")));
//!
//! assert_eq!(buffered.next().await, None);
//! # Ok::<(), &'static str>(()) }).unwrap();
//! ```

mod buffered_weighted_stream;
mod global_weight;
mod memory_bound;
mod peekable_fused;
#[cfg(test)]
mod tests;

pub use crate::buffered_weighted_stream::BufferedWeighted;
pub use crate::memory_bound::MemoryBound;

/// Traits to aid in type definitions.
///
/// These traits are normally not required by end-user code, but may be necessary for some generic
/// code.
pub mod traits {
    pub use crate::buffered_weighted_stream::WeightedFuture;
}

use futures_util::Future;
use futures_util::Stream;

impl<T: ?Sized> StreamExt for T where T: Stream {}

/// An extension trait for `Stream`s that provides
/// [`buffered_weighted`](StreamExt::buffered_weighted) and [`buffered_weighted_bounded`](StreamExt::buffered_weighted_bounded).
pub trait StreamExt: Stream {
    /// An adaptor for creating an ordered queue of pending futures, where each future has a
    /// different weight.
    ///
    /// This stream must return values of type `(usize, impl Future)`, where the `usize` indicates
    /// the weight of each future. This adaptor will buffer futures up to weight `max_weight`, and
    /// then return the outputs in the order in which they complete.
    ///
    /// * The maximum weight is never exceeded while futures are being run.
    /// * If the weight of an individual future is greater than the maximum weight, its weight will
    ///   be set to the maximum weight.
    ///
    /// The adaptor will schedule futures in the order they're returned by the stream, without doing
    /// any reordering based on weight.
    ///
    /// The weight of a future can be zero, in which case it will not count towards the total weight.
    ///
    /// The returned stream will be a stream of each future's output.
    ///
    /// # Examples
    ///
    /// See [the crate documentation](crate#examples) for an example.
    fn buffered_weighted<Fut>(self, max_weight: usize) -> BufferedWeighted<Self>
    where
        Self: Sized + Stream<Item = (usize, Fut)>,
        Fut: Future,
    {
        assert_stream::<Fut::Output, _>(BufferedWeighted::new(self, max_weight, None))
    }

    /// An adaptor for creating an ordered queue of pending futures, where each future has a
    /// different weight.
    ///
    /// This stream must return values of type `(usize, impl Future)`, where the `usize` indicates
    /// the weight of each future. This adaptor will buffer futures up to weight `max_weight`, and
    /// then return the outputs in the order in which they complete. In addition to the constraint of
    /// `max_weight`, this adaptor will also enforce a memory bound before scheduling any new future for
    /// execution. The memory bound serves as a free memory limit that the combinator must honor while
    /// scheduling new futures in the stream.
    fn buffered_weighted_bounded<Fut>(
        self,
        max_weight: usize,
        memory_bound: u64,
    ) -> BufferedWeighted<Self>
    where
        Self: Sized + Stream<Item = (usize, Fut)>,
        Fut: Future,
    {
        assert_stream::<Fut::Output, _>(BufferedWeighted::new(self, max_weight, Some(memory_bound)))
    }
}

pub(crate) fn assert_stream<T, S>(stream: S) -> S
where
    S: Stream<Item = T>,
{
    stream
}
