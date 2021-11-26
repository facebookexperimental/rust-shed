/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! An implementation of `futures_stats` for Futures 0.1.

use futures_01_ext::{BoxFuture, BoxFutureNonSend, FutureExt};
use futures_old::{Async, Future, IntoFuture, Poll};
use std::time::{Duration, Instant};

use super::FutureStats;

/// A Future that gathers some basic statistics for inner Future.
/// This structure main usage is by calling [Timed::timed].
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
    type Item = (Result<F::Item, F::Error>, FutureStats);
    type Error = !;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let _ = self.start.get_or_insert_with(Instant::now);
        self.poll_count += 1;

        let poll_start = Instant::now();
        let poll = self.inner.poll();
        self.poll_time += poll_start.elapsed();

        let res = match poll {
            Ok(Async::NotReady) => return Ok(Async::NotReady),
            Ok(Async::Ready(v)) => Ok(v),
            Err(e) => Err(e),
        };

        let stats = FutureStats {
            completion_time: self.start.expect("start time not set").elapsed(),
            poll_time: self.poll_time,
            poll_count: self.poll_count,
        };

        Ok(Async::Ready((res, stats)))
    }
}

fn time_future<F, C, R>(future: F, callback: C) -> impl Future<Item = F::Item, Error = F::Error>
where
    F: Future,
    C: FnOnce(FutureStats, Result<&F::Item, &F::Error>) -> R,
    R: IntoFuture<Item = (), Error = ()> + 'static,
    R::Future: 'static,
{
    TimedFuture::new(future).then(|res| {
        let (res, stats) = res.expect("unexpected unreachable err");
        callback(stats, res.as_ref()).into_future().then(|_| res)
    })
}

fn future_with_timing<F>(
    future: F,
) -> impl Future<Item = (FutureStats, F::Item), Error = (FutureStats, F::Error)>
where
    F: Future,
{
    TimedFuture::new(future).then(|res| {
        let (real_res, stats) = res.expect("unexpected unreachable err");
        match real_res {
            Ok(r) => Ok((stats, r)),
            Err(e) => Err((stats, e)),
        }
    })
}

/// A trait that provides extra methods to [futures_old::Future] used for gathering stats
pub trait Timed: Future + Sized + Send + 'static {
    /// Combinator that returns a future that will gather some statistics using
    /// [TimedFuture] and pass them for inspection to the provided callback.
    fn timed<C, R>(self, callback: C) -> BoxFuture<Self::Item, Self::Error>
    where
        C: FnOnce(FutureStats, Result<&Self::Item, &Self::Error>) -> R + Send + 'static,
        R: IntoFuture<Item = (), Error = ()> + 'static,
        R::Future: Send + 'static,
        Self::Item: Send,
        Self::Error: Send,
    {
        time_future(self, callback).boxify()
    }

    /// Combinator that returns a future that will gether some statistics using
    /// [TimedFuture] and return them together with the result of inner future.
    fn collect_timing(self) -> BoxFuture<(FutureStats, Self::Item), (FutureStats, Self::Error)>
    where
        Self::Item: Send,
        Self::Error: Send,
    {
        future_with_timing(self).boxify()
    }
}

/// Similar to [Timed], but adds the extra methods to NonSend Futures
pub trait TimedNonSend: Future + Sized + 'static {
    /// Combinator that returns a future that will gather some statistics using
    /// [TimedFuture] and pass them for inspection to the provided callback.
    fn timed_nonsend<C, R>(self, callback: C) -> BoxFutureNonSend<Self::Item, Self::Error>
    where
        C: FnOnce(FutureStats, Result<&Self::Item, &Self::Error>) -> R + 'static,
        R: IntoFuture<Item = (), Error = ()> + 'static,
        R::Future: 'static,
    {
        time_future(self, callback).boxify_nonsend()
    }

    /// Combinator that returns a future that will gether some statistics using
    /// [TimedFuture] and return them together with the result of inner future.
    fn collect_timing(
        self,
    ) -> BoxFutureNonSend<(FutureStats, Self::Item), (FutureStats, Self::Error)> {
        future_with_timing(self).boxify_nonsend()
    }
}

impl<T: Future + Send + 'static> Timed for T {}
impl<T: Future + 'static> TimedNonSend for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_old::future::{err, ok};

    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_collect_timings_with_future_ok() {
        let result_ok = Arc::new(AtomicBool::new(false));

        let f: BoxFuture<u32, ()> = ok(123).boxify();

        let f = Timed::collect_timing(f)
            .map({
                let result_ok = result_ok.clone();
                move |(_, r)| {
                    result_ok.store(r == 123, Ordering::SeqCst);
                    ()
                }
            })
            .map(|_| ())
            .map_err(|_| ())
            .boxify();

        tokio_old::run(f);
        assert!(result_ok.load(Ordering::SeqCst));
    }

    #[test]
    fn test_collect_timings_with_future_error() {
        let err_ok = Arc::new(AtomicBool::new(false));

        let f: BoxFuture<(), u32> = err(123).boxify();

        let f = Timed::collect_timing(f)
            .map_err({
                let err_ok = err_ok.clone();
                move |(_, r)| {
                    err_ok.store(r == 123, Ordering::SeqCst);
                    ()
                }
            })
            .map(|_| ())
            .map_err(|_| ())
            .boxify();

        tokio_old::run(f);
        assert!(err_ok.load(Ordering::SeqCst));
    }
}
