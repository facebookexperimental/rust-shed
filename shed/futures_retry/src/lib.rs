/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Retry capabilities for futures.
//!
//! This provides a generic method for retrying fallible futures (i.e.
//! TryFutures).  It supports various kinds of backoff, customizable with the
//! builder pattern.

use std::future::Future;
use std::pin::Pin;
use std::task::Poll;
use std::time::Duration;

use pin_project::pin_project;

use crate::backoff::ExponentialBackoff;
use crate::backoff::FibonacciBackoff;
use crate::backoff::FixedInterval;
use crate::backoff::Jitter;

pub mod backoff;

pub trait InspectErr<E>: Send {
    fn inspect_err(&mut self, attempt: usize, err: &E);
}

impl<E> InspectErr<E> for () {
    fn inspect_err(&mut self, _attempt: usize, _err: &E) {}
}

impl<E, F> InspectErr<E> for F
where
    F: for<'err> FnMut(usize, &'err E) + Send,
{
    fn inspect_err(&mut self, attempt: usize, err: &E) {
        self(attempt, err);
    }
}

pub trait RetryIf<E>: Send {
    fn retry_if(&mut self, attempt: usize, err: &E) -> bool;
}

impl<E> RetryIf<E> for () {
    fn retry_if(&mut self, _attempt: usize, _err: &E) -> bool {
        true
    }
}

impl<E, F> RetryIf<E> for F
where
    F: for<'err> FnMut(usize, &'err E) -> bool + Send,
{
    fn retry_if(&mut self, attempt: usize, err: &E) -> bool {
        self(attempt, err)
    }
}

#[pin_project]
pub struct Retry<F, Fut, V, E, B, I, R>
where
    F: FnMut(usize) -> Fut + Send,
    Fut: Future<Output = Result<V, E>>,
    V: Send + 'static,
    B: Iterator<Item = Duration> + Send,
    I: InspectErr<E>,
    R: RetryIf<E>,
{
    func: F,
    #[pin]
    fut: Option<Fut>,
    #[pin]
    sleep: Option<tokio::time::Sleep>,
    attempt: usize,
    backoff: B,
    max_attempts: Option<usize>,
    max_interval: Option<Duration>,
    inspect_err: I,
    retry_if: R,
}

/// Retry a fallible future if it fails.
///
/// By default, retries will happen after the given interval, and will
/// continue until the future is successful.
///
/// You can customize the behaviour with the methods on `Retry`.
pub fn retry<F, Fut, V, E>(
    func: F,
    interval: Duration,
) -> Retry<F, Fut, V, E, FixedInterval, (), ()>
where
    F: FnMut(usize) -> Fut + Send,
    Fut: Future<Output = Result<V, E>>,
    V: Send + 'static,
{
    Retry {
        func,
        fut: None,
        sleep: None,
        attempt: 0,
        backoff: FixedInterval::new(interval),
        max_attempts: None,
        max_interval: None,
        inspect_err: (),
        retry_if: (),
    }
}

impl<F, Fut, V, E, B, I, R> Retry<F, Fut, V, E, B, I, R>
where
    F: FnMut(usize) -> Fut + Send,
    Fut: Future<Output = Result<V, E>>,
    V: Send + 'static,
    E: Send + 'static,
    B: Iterator<Item = Duration> + Send,
    I: InspectErr<E>,
    R: RetryIf<E>,
{
    /// Limit the number of attempts.  If the last attempt fails then the
    /// error is returned.
    pub fn max_attempts(mut self, max_attempts: usize) -> Self {
        self.max_attempts = Some(max_attempts);
        self
    }

    /// Limit the interval to a maximum value.
    pub fn max_interval(mut self, max_interval: Duration) -> Self {
        self.max_interval = Some(max_interval);
        self
    }

    /// Perform binary exponential backoff.  The second and
    /// subsequent retry intervals will be multiplied by
    /// 2, 4, 8, etc.
    pub fn binary_exponential_backoff(mut self) -> Retry<F, Fut, V, E, ExponentialBackoff, I, R>
    where
        B: Iterator<Item = Duration>,
    {
        let initial_interval = self.backoff.next().unwrap_or(Duration::from_millis(10));
        Retry {
            func: self.func,
            fut: self.fut,
            sleep: self.sleep,
            attempt: self.attempt,
            backoff: ExponentialBackoff::binary(initial_interval),
            max_attempts: self.max_attempts,
            max_interval: self.max_interval,
            inspect_err: self.inspect_err,
            retry_if: self.retry_if,
        }
    }

    /// Perform exponential backoff with the given base.
    /// The second and subsequent retry intervals will be
    /// multiplied by base, base^2^, base^3^, etc.
    pub fn exponential_backoff(
        mut self,
        base: impl Into<f64>,
    ) -> Retry<F, Fut, V, E, ExponentialBackoff, I, R>
    where
        B: Iterator<Item = Duration>,
    {
        let initial_interval = self.backoff.next().unwrap_or(Duration::from_millis(10));
        Retry {
            func: self.func,
            fut: self.fut,
            sleep: self.sleep,
            attempt: self.attempt,
            backoff: ExponentialBackoff::new(initial_interval, base.into()),
            max_attempts: self.max_attempts,
            max_interval: self.max_interval,
            inspect_err: self.inspect_err,
            retry_if: self.retry_if,
        }
    }

    /// Perform fibonacci backoff.  Each retry interval will
    /// be the sum of the previous two intervals.
    pub fn fibonacci_backoff(mut self) -> Retry<F, Fut, V, E, FibonacciBackoff, I, R>
    where
        B: Iterator<Item = Duration>,
    {
        let initial_interval = self.backoff.next().unwrap_or(Duration::from_millis(10));
        Retry {
            func: self.func,
            fut: self.fut,
            sleep: self.sleep,
            attempt: self.attempt,
            backoff: FibonacciBackoff::new(initial_interval),
            max_attempts: self.max_attempts,
            max_interval: self.max_interval,
            inspect_err: self.inspect_err,
            retry_if: self.retry_if,
        }
    }

    /// Add jitter to the retry intervals.  The additional delay is
    /// uniformly random between zero and the jitter duration.
    pub fn jitter(self, jitter: Duration) -> Retry<F, Fut, V, E, Jitter<B>, I, R>
    where
        B: Iterator<Item = Duration> + Send,
    {
        Retry {
            func: self.func,
            fut: self.fut,
            sleep: self.sleep,
            attempt: self.attempt,
            backoff: Jitter::new(self.backoff, jitter),
            max_attempts: self.max_attempts,
            max_interval: self.max_interval,
            inspect_err: self.inspect_err,
            retry_if: self.retry_if,
        }
    }

    /// Inspect each error that occurs.  The closure is called after each
    /// attempt that fails, allowing you to log the error.
    pub fn inspect_err<I2>(self, inspect_err: I2) -> Retry<F, Fut, V, E, B, I2, R>
    where
        I2: FnMut(usize, &E) + Send,
    {
        Retry {
            func: self.func,
            fut: self.fut,
            sleep: self.sleep,
            attempt: self.attempt,
            backoff: self.backoff,
            max_attempts: self.max_attempts,
            max_interval: self.max_interval,
            inspect_err,
            retry_if: self.retry_if,
        }
    }

    /// Add a condition to retrying.  If the closure returns false then the
    /// error is returned instead of retrying.
    pub fn retry_if<R2>(self, retry_if: R2) -> Retry<F, Fut, V, E, B, I, R2>
    where
        R2: FnMut(usize, &E) -> bool + Send,
    {
        Retry {
            func: self.func,
            fut: self.fut,
            sleep: self.sleep,
            attempt: self.attempt,
            backoff: self.backoff,
            max_attempts: self.max_attempts,
            max_interval: self.max_interval,
            inspect_err: self.inspect_err,
            retry_if,
        }
    }
}

impl<F, Fut, V, E, B, I, R> Future for Retry<F, Fut, V, E, B, I, R>
where
    F: FnMut(usize) -> Fut + Send,
    Fut: Future<Output = Result<V, E>>,
    V: Send + 'static,
    E: Send + 'static,
    B: Iterator<Item = Duration> + Send,
    I: InspectErr<E>,
    R: RetryIf<E>,
{
    type Output = Result<(V, usize), E>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();
        loop {
            if let Some(fut) = this.fut.as_mut().as_pin_mut() {
                match fut.poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(Ok(v)) => {
                        return Poll::Ready(Ok((v, *this.attempt)));
                    }
                    Poll::Ready(Err(e)) => {
                        this.fut.set(None);
                        this.inspect_err.inspect_err(*this.attempt, &e);
                        if this.max_attempts.is_none_or(|max| *this.attempt < max)
                            && this.retry_if.retry_if(*this.attempt, &e)
                            && let Some(mut interval) = this.backoff.next()
                        {
                            if let Some(max_interval) = this.max_interval {
                                interval = interval.clamp(Duration::ZERO, *max_interval);
                            }
                            this.sleep.set(Some(tokio::time::sleep(interval)));
                        } else {
                            return Poll::Ready(Err(e));
                        }
                    }
                }
            }
            if let Some(sleep) = this.sleep.as_mut().as_pin_mut() {
                match sleep.poll(cx) {
                    Poll::Pending => return Poll::Pending,
                    Poll::Ready(()) => {
                        this.sleep.set(None);
                    }
                }
            }
            *this.attempt += 1;
            this.fut.set(Some((this.func)(*this.attempt)));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    enum TestError {
        NotYet,
        AlwaysFails,
    }

    #[tokio::test]
    async fn test_retry_success_first_attempt() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = retry(
            move |attempt| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, TestError>(format!("success on attempt {}", attempt))
                }
            },
            Duration::from_millis(10),
        )
        .await;

        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert!(result.is_ok());
        let (value, attempts) = result.unwrap();
        assert_eq!(value, "success on attempt 1");
        assert_eq!(attempts, 1);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = retry(
            move |attempt| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    if attempt < 3 {
                        Err(TestError::NotYet)
                    } else {
                        Ok(format!("success on attempt {}", attempt))
                    }
                }
            },
            Duration::from_millis(10),
        )
        .await;

        assert_eq!(counter.load(Ordering::SeqCst), 3);
        assert!(result.is_ok());
        let (value, attempts) = result.unwrap();
        assert_eq!(value, "success on attempt 3");
        assert_eq!(attempts, 3);
    }

    #[tokio::test]
    async fn test_retry_max_attempts() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = retry(
            move |_attempt| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err::<String, _>(TestError::AlwaysFails)
                }
            },
            Duration::from_millis(10),
        )
        .max_attempts(3)
        .await;

        assert_eq!(counter.load(Ordering::SeqCst), 3);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), TestError::AlwaysFails);
    }

    #[tokio::test]
    async fn test_retry_with_exponential_backoff() {
        let start_time = std::time::Instant::now();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = retry(
            move |attempt| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    if attempt < 3 {
                        Err(TestError::NotYet)
                    } else {
                        Ok(format!("success on attempt {}", attempt))
                    }
                }
            },
            Duration::from_millis(10),
        )
        .binary_exponential_backoff()
        .await;

        let elapsed = start_time.elapsed();

        // We expect at least:
        // - First attempt: immediate
        // - Second attempt: after 10ms
        // - Third attempt: after 20ms (10ms * 2)
        // Total: at least 30ms
        assert!(elapsed >= Duration::from_millis(30));

        assert_eq!(counter.load(Ordering::SeqCst), 3);
        assert!(result.is_ok());
        let (value, attempts) = result.unwrap();
        assert_eq!(value, "success on attempt 3");
        assert_eq!(attempts, 3);
    }

    #[tokio::test]
    async fn test_retry_with_fibonacci_backoff() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = retry(
            move |attempt| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    if attempt < 4 {
                        Err(TestError::NotYet)
                    } else {
                        Ok(format!("success on attempt {}", attempt))
                    }
                }
            },
            Duration::from_millis(10),
        )
        .fibonacci_backoff()
        .await;

        assert_eq!(counter.load(Ordering::SeqCst), 4);
        assert!(result.is_ok());
        let (value, attempts) = result.unwrap();
        assert_eq!(value, "success on attempt 4");
        assert_eq!(attempts, 4);
    }

    #[tokio::test]
    async fn test_retry_with_max_interval() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = retry(
            move |attempt| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    if attempt < 4 {
                        Err(TestError::NotYet)
                    } else {
                        Ok(format!("success on attempt {}", attempt))
                    }
                }
            },
            Duration::from_millis(10),
        )
        .binary_exponential_backoff()
        .max_interval(Duration::from_millis(15))
        .await;

        assert_eq!(counter.load(Ordering::SeqCst), 4);
        assert!(result.is_ok());
        let (value, attempts) = result.unwrap();
        assert_eq!(value, "success on attempt 4");
        assert_eq!(attempts, 4);
    }

    #[tokio::test]
    async fn test_retry_with_jitter() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let result = retry(
            move |attempt| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    if attempt < 3 {
                        Err(TestError::NotYet)
                    } else {
                        Ok(format!("success on attempt {}", attempt))
                    }
                }
            },
            Duration::from_millis(10),
        )
        .jitter(Duration::from_millis(5))
        .await;

        assert_eq!(counter.load(Ordering::SeqCst), 3);
        assert!(result.is_ok());
        let (value, attempts) = result.unwrap();
        assert_eq!(value, "success on attempt 3");
        assert_eq!(attempts, 3);
    }

    #[tokio::test]
    async fn test_retry_with_inspect_err() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let errors = Arc::new(AtomicUsize::new(0));
        let errors_clone = errors.clone();

        let result = retry(
            move |attempt| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    if attempt < 3 {
                        Err(TestError::NotYet)
                    } else {
                        Ok(format!("success on attempt {}", attempt))
                    }
                }
            },
            Duration::from_millis(10),
        )
        .inspect_err(move |_attempt, _err: &TestError| {
            errors_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

        assert_eq!(counter.load(Ordering::SeqCst), 3);
        assert_eq!(errors.load(Ordering::SeqCst), 2); // 2 errors before success
        assert!(result.is_ok());
        let (value, attempts) = result.unwrap();
        assert_eq!(value, "success on attempt 3");
        assert_eq!(attempts, 3);
    }

    #[tokio::test]
    async fn test_retry_with_retry_if() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Create a retry condition that only retries on even-numbered attempts
        let result = retry(
            move |_attempt| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err::<String, TestError>(TestError::AlwaysFails)
                }
            },
            Duration::from_millis(10),
        )
        .retry_if(|attempt, _err| attempt < 3) // Only retry for the first 3 attempts
        .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 3); // Should have tried 3 times
        assert_eq!(result.unwrap_err(), TestError::AlwaysFails);
    }

    #[tokio::test]
    async fn test_retry_with_retry_if_error_type() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        // Create a retry condition that only retries for certain error types
        let result = retry::<_, _, String, TestError>(
            move |_attempt| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    if c.load(Ordering::SeqCst) % 2 == 1 {
                        Err(TestError::NotYet) // Retry on this error
                    } else {
                        Err(TestError::AlwaysFails) // Don't retry on this error
                    }
                }
            },
            Duration::from_millis(10),
        )
        .retry_if(|_attempt, err| matches!(err, TestError::NotYet)) // Only retry for NotYet errors
        .await;

        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 2); // Should have tried twice (1st attempt gets AlwaysFails, no retry)
        assert_eq!(result.unwrap_err(), TestError::AlwaysFails);
    }

    #[tokio::test]
    async fn test_retry_builder_chaining() {
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();

        let errors = Arc::new(AtomicUsize::new(0));
        let errors_clone = errors.clone();

        let result = retry(
            move |attempt| {
                let c = counter_clone.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    if attempt < 3 {
                        Err(TestError::NotYet)
                    } else {
                        Ok(format!("success on attempt {}", attempt))
                    }
                }
            },
            Duration::from_millis(10),
        )
        .binary_exponential_backoff()
        .max_attempts(5)
        .max_interval(Duration::from_millis(50))
        .jitter(Duration::from_millis(5))
        .inspect_err(move |_attempt, _err: &TestError| {
            errors_clone.fetch_add(1, Ordering::SeqCst);
        })
        .await;

        assert_eq!(counter.load(Ordering::SeqCst), 3);
        assert_eq!(errors.load(Ordering::SeqCst), 2);
        assert!(result.is_ok());
        let (value, attempts) = result.unwrap();
        assert_eq!(value, "success on attempt 3");
        assert_eq!(attempts, 3);
    }
}
