/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! An implementation of `futures_stats` for Futures 0.1.

use futures_01_ext::{BoxFuture, BoxFutureNonSend, BoxStream, FutureExt, StreamExt};
use futures_old::{Async, Future, IntoFuture, Poll, Stream};
use std::time::{Duration, Instant};

use super::{FutureStats, StreamStats};

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

/// A Stream that gathers some basic statistics for inner Stream.
/// This structure main usage is by calling [TimedStreamTrait::timed].
pub struct TimedStream<S, C, R>
where
    R: IntoFuture<Item = (), Error = ()> + 'static,
    S: Stream,
{
    inner: S,
    callback: Option<C>,
    callback_future: Option<R::Future>,
    start: Option<Instant>,
    stream_result: Option<Result<(), S::Error>>,
    count: usize,
    poll_count: u64,
    poll_time: Duration,
    first_item_time: Option<Duration>,
}

impl<S, C, R> TimedStream<S, C, R>
where
    R: IntoFuture<Item = (), Error = ()> + 'static,
    S: Stream,
{
    fn new(stream: S, callback: C) -> Self {
        TimedStream {
            inner: stream,
            callback: Some(callback),
            callback_future: None,
            start: None,
            stream_result: None,
            count: 0,
            poll_count: 0,
            poll_time: Duration::from_secs(0),
            first_item_time: None,
        }
    }
}

impl<S, C, R> Stream for TimedStream<S, C, R>
where
    S: Stream,
    C: FnOnce(StreamStats, Result<(), &S::Error>) -> R,
    R: IntoFuture<Item = (), Error = ()> + 'static,
{
    type Item = S::Item;
    type Error = S::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if self.callback_future.is_some() {
            // We've already exhausted the stream, now we are just processing callback future
            return self.poll_callback_future();
        }

        let _ = self.start.get_or_insert_with(Instant::now);
        self.poll_count += 1;

        let poll_start = Instant::now();
        let poll = self.inner.poll();
        self.poll_time += poll_start.elapsed();
        match poll {
            Ok(Async::NotReady) => Ok(Async::NotReady),
            notfinished @ Ok(Async::Ready(Some(_))) => {
                self.count += 1;
                if self.count == 1 {
                    self.first_item_time = Some(self.start.expect("start time not set").elapsed());
                }
                notfinished
            }
            Ok(Async::Ready(None)) => {
                let callback_future = self.run_callback(Ok(()));
                self.stream_result = Some(Ok(()));
                self.callback_future = Some(callback_future.into_future());
                self.poll_callback_future()
            }
            Err(err) => {
                let callback_future = self.run_callback(Err(&err));
                self.stream_result = Some(Err(err));
                self.callback_future = Some(callback_future.into_future());
                self.poll_callback_future()
            }
        }
    }
}

impl<S, C, R> TimedStream<S, C, R>
where
    S: Stream,
    C: FnOnce(StreamStats, Result<(), &S::Error>) -> R,
    R: IntoFuture<Item = (), Error = ()> + 'static,
{
    fn run_callback(&mut self, res: Result<(), &S::Error>) -> R {
        let stats = StreamStats {
            completion_time: self.start.expect("start time not set").elapsed(),
            poll_time: self.poll_time,
            poll_count: self.poll_count,
            count: self.count,
            first_item_time: self.first_item_time,
        };
        let callback = self.callback.take().expect("callback was already called");
        callback(stats, res)
    }

    fn poll_callback_future(
        &mut self,
    ) -> Poll<Option<<Self as Stream>::Item>, <Self as Stream>::Error> {
        if let Some(ref mut fut) = self.callback_future {
            // We've already exhausted the stream, now we are just processing callback future
            let poll = fut.poll();
            if poll == Ok(Async::NotReady) {
                return Ok(Async::NotReady);
            }

            let stream_result = self
                .stream_result
                .take()
                .expect("stream result should have been set");

            match stream_result {
                Ok(()) => Ok(Async::Ready(None)),
                Err(err) => Err(err),
            }
        } else {
            panic!("callback future is not set!");
        }
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

/// A trait that provides extra methods to [futures_old::Stream] used for gathering stats
pub trait TimedStreamTrait: Stream + Sized + Send + 'static {
    /// Combinator that returns a stream that will gather some statistics using
    /// [TimedStream] and pass them for inspection to the provided callback.
    fn timed<C, R>(self, callback: C) -> BoxStream<Self::Item, Self::Error>
    where
        C: FnOnce(StreamStats, Result<(), &Self::Error>) -> R + Send + 'static,
        R: IntoFuture<Item = (), Error = ()> + Send + 'static,
        R::Future: 'static,
        <R as futures_old::IntoFuture>::Future: Send,
        Self::Item: Send,
        Self::Error: Send,
    {
        TimedStream::new(self, callback).boxify()
    }
}

impl<T: Stream + Send + 'static> TimedStreamTrait for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Error;
    use futures_old::future::{err, ok};
    use futures_old::stream::{iter_ok, once};

    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_timed_stream_simple() {
        let callback_called = Arc::new(AtomicBool::new(false));

        const TEST_COUNT: usize = 3;
        let s: BoxStream<_, ()> = iter_ok([0; TEST_COUNT].iter())
            .timed({
                let callback_called = callback_called.clone();
                move |stats, _| {
                    assert_eq!(stats.count, TEST_COUNT);
                    callback_called.store(true, Ordering::SeqCst);
                    Ok(())
                }
            })
            .boxify();

        tokio_old::run(s.collect().map(|_| ()));
        assert!(callback_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_timed_stream_error() {
        let callback_called = Arc::new(AtomicBool::new(false));
        let err_happened = Arc::new(AtomicBool::new(false));
        let err_reported = Arc::new(AtomicBool::new(false));

        let s: BoxStream<(), _> = once(Err(Error::msg("err")))
            .timed({
                let callback_called = callback_called.clone();
                let err_reported = err_reported.clone();
                move |_, res| {
                    callback_called.store(true, Ordering::SeqCst);
                    err_reported.store(res.is_err(), Ordering::SeqCst);
                    Ok(())
                }
            })
            .boxify();

        tokio_old::run(s.collect().map(|_| ()).map_err({
            let err_happened = err_happened.clone();
            move |_| err_happened.store(true, Ordering::SeqCst)
        }));
        assert!(callback_called.load(Ordering::SeqCst));
        assert!(err_happened.load(Ordering::SeqCst));
        assert!(err_reported.load(Ordering::SeqCst));
    }

    #[test]
    fn test_timed_with_future() {
        let sleep_fut = tokio_timer::sleep(Duration::from_millis(300));

        let future_called = Arc::new(AtomicBool::new(false));
        let s: BoxStream<_, ()> = iter_ok([1, 2, 3].iter())
            .timed({
                let future_called = future_called.clone();
                move |_, _| {
                    sleep_fut
                        .map(move |_| {
                            future_called.store(true, Ordering::SeqCst);
                            ()
                        })
                        .map_err(|_| ())
                }
            })
            .boxify();

        tokio_old::run(s.collect().map(|_| ()));
        assert!(future_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_timed_with_err_and_future() {
        let sleep_fut = tokio_timer::sleep(Duration::from_millis(300));

        let future_called = Arc::new(AtomicBool::new(false));
        let err_happened = Arc::new(AtomicBool::new(false));
        let err_reported = Arc::new(AtomicBool::new(false));
        let s: BoxStream<(), _> = once(Err(Error::msg("err")))
            .timed({
                let err_reported = err_reported.clone();
                let future_called = future_called.clone();
                move |_, res| {
                    err_reported.store(res.is_err(), Ordering::SeqCst);
                    sleep_fut
                        .map(move |_| {
                            future_called.store(true, Ordering::SeqCst);
                            ()
                        })
                        .map_err(|_| ())
                }
            })
            .boxify();

        tokio_old::run(s.collect().map(|_| ()).map_err({
            let err_happened = err_happened.clone();
            move |_| {
                err_happened.store(true, Ordering::SeqCst);
                ()
            }
        }));
        assert!(err_happened.load(Ordering::SeqCst));
        assert!(err_reported.load(Ordering::SeqCst));
        assert!(future_called.load(Ordering::SeqCst));
    }

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
