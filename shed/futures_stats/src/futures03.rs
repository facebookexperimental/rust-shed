/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

//! An implementation of `futures_stats` for Futures 0.3.

use std::future::Future;

use futures_preview::stream::Stream;
use futures_preview::task::{Context, Poll};
use std::pin::Pin;
use std::time::{Duration, Instant};

use super::{FutureStats, StreamStats};

/// A Future that gathers some basic statistics for inner Future.
/// This structure's main usage is by calling [TimedFutureExt::timed].
pub struct TimedFuture<F> {
    inner: F,
    start: Option<Instant>,
    poll_count: u64,
    poll_time: Duration,
}

impl<F> TimedFuture<F> {
    fn new(future: F) -> Self {
        TimedFuture {
            inner: future,
            start: None,
            poll_count: 0,
            poll_time: Duration::from_secs(0),
        }
    }
}

impl<F: Future> Future for TimedFuture<F> {
    type Output = (FutureStats, F::Output);

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let _ = this.start.get_or_insert_with(Instant::now);
        this.poll_count += 1;

        let poll_start = Instant::now();

        let poll = unsafe { Pin::new_unchecked(&mut this.inner).poll(cx) };
        this.poll_time += poll_start.elapsed();

        let out = match poll {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(v) => v,
        };

        let stats = FutureStats {
            completion_time: this.start.expect("start time not set").elapsed(),
            poll_time: this.poll_time,
            poll_count: this.poll_count,
        };

        Poll::Ready((stats, out))
    }
}

/// A Stream that gathers some basic statistics for inner Stream.
/// This structure's main usage is by calling [TimedStreamExt::timed].
pub struct TimedStream<S, C, F>
where
    S: Stream,
    C: FnOnce(StreamStats) -> F,
    F: Future<Output = ()>,
{
    inner: S,
    callback: Option<C>,
    callback_future: Option<F>,
    start: Option<Instant>,
    count: usize,
    poll_count: u64,
    poll_time: Duration,
}

impl<S, C, F> TimedStream<S, C, F>
where
    S: Stream,
    C: FnOnce(StreamStats) -> F,
    F: Future<Output = ()>,
{
    fn new(stream: S, callback: C) -> Self {
        TimedStream {
            inner: stream,
            callback: Some(callback),
            callback_future: None,
            start: None,
            count: 0,
            poll_count: 0,
            poll_time: Duration::from_secs(0),
        }
    }

    fn run_callback(&mut self) -> F {
        let stats = StreamStats {
            completion_time: self.start.expect("start time not set").elapsed(),
            poll_time: self.poll_time,
            poll_count: self.poll_count,
            count: self.count,
        };
        let callback = self.callback.take().expect("callback was already called");
        callback(stats)
    }

    fn poll_callback_future(&mut self, cx: &mut Context) -> Poll<Option<<Self as Stream>::Item>> {
        if let Some(ref mut fut) = self.callback_future {
            // We've already exhausted the stream, now we are just processing callback future
            let poll = unsafe { Pin::new_unchecked(fut).poll(cx) };
            match poll {
                Poll::Pending => Poll::Pending,
                Poll::Ready(()) => Poll::Ready(None),
            }
        } else {
            panic!("callback future is not set!");
        }
    }
}

impl<S, C, F> Stream for TimedStream<S, C, F>
where
    S: Stream,
    C: FnOnce(StreamStats) -> F,
    F: Future<Output = ()>,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = unsafe { self.get_unchecked_mut() };
        if this.callback_future.is_some() {
            // We've already exhausted the stream, now we are just processing callback future
            return this.poll_callback_future(cx);
        }

        let _ = this.start.get_or_insert_with(Instant::now);
        this.poll_count += 1;

        let poll_start = Instant::now();
        let poll = unsafe { Pin::new_unchecked(&mut this.inner).poll_next(cx) };
        this.poll_time += poll_start.elapsed();
        match poll {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Some(item)) => {
                this.count += 1;
                Poll::Ready(Some(item))
            }
            Poll::Ready(None) => {
                this.callback_future = Some(this.run_callback());
                this.poll_callback_future(cx)
            }
        }
    }
}

/// A trait that provides the `timed` method to [futures::Future] for gathering stats
pub trait TimedFutureExt: Future + Sized {
    /// Combinator that returns a future that will gather some statistics and
    /// return them together with the result of inner future.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_stats::TimedFutureExt;
    ///
    /// # futures_preview::executor::block_on(async {
    /// let (stats, value) = async { 123u32 }.timed().await;
    /// assert_eq!(value, 123);
    /// assert!(stats.poll_count > 0);
    /// # });
    /// ```
    fn timed(self) -> TimedFuture<Self> {
        TimedFuture::new(self)
    }
}

impl<T: Future> TimedFutureExt for T {}

/// A trait that provides the `timed` method to [futures::Stream] for gathering stats
pub trait TimedStreamExt: Stream + Sized {
    /// Combinator that returns a stream that will gather some statistics and
    /// pass them for inspection to the provided callback when the stream
    /// completes.
    ///
    /// # Examples
    ///
    /// ```
    /// use futures_stats::TimedStreamExt;
    /// use futures_preview::stream::{self, StreamExt};
    ///
    /// # futures_preview::executor::block_on(async {
    /// let out = stream::iter([0u32; 3].iter())
    ///     .timed(|stats| {
    ///         async move {
    ///             assert_eq!(stats.count, 3);
    ///         }
    ///     })
    ///     .collect::<Vec<u32>>()
    ///     .await;
    /// assert_eq!(out, vec![0, 0, 0]);
    /// # });
    /// ```
    fn timed<C, F>(self, callback: C) -> TimedStream<Self, C, F>
    where
        C: FnOnce(StreamStats) -> F,
        F: Future<Output = ()>,
    {
        TimedStream::new(self, callback)
    }
}

impl<T: Stream> TimedStreamExt for T {}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use futures_preview::stream::{self, StreamExt};
    use tokio_preview as tokio;

    #[tokio::test]
    async fn test_timed_future() {
        let (stats, result) = async { 123u32 }.timed().await;
        assert_eq!(result, 123u32);
        assert!(stats.poll_count > 0);
    }

    #[tokio::test]
    async fn test_timed_stream() {
        let callback_called = Arc::new(AtomicBool::new(false));
        const TEST_COUNT: usize = 3;
        let out: Vec<_> = stream::iter([0u32; TEST_COUNT].iter())
            .timed({
                let callback_called = callback_called.clone();
                move |stats| {
                    async move {
                        assert_eq!(stats.count, TEST_COUNT);
                        callback_called.store(true, Ordering::SeqCst);
                    }
                }
            })
            .collect::<Vec<u32>>()
            .await;
        assert_eq!(out, vec![0; TEST_COUNT]);
        assert!(callback_called.load(Ordering::SeqCst));
    }
}
