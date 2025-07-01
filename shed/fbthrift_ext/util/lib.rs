/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;

use tokio::sync::Mutex;
use tokio::sync::MutexGuard;

/// Future for the [`poll_with_lock`] function.
pub struct PollWithLock<'a, T, F, E>
where
    F: FnMut(&mut MutexGuard<'a, T>, &mut Context<'_>) -> Poll<Result<(), E>> + Unpin,
{
    lock: &'a Mutex<T>,
    f: F,
}

/// Creates a new future that supports polling another future behind [`tokio::sync::Mutex`].
///
/// When some future-like struct is behind a lock, polling with the mutex locked easily creates
/// deadlock since no one else will be able to make progress on the lock-protected struct.
///
/// Note the closure should return a `Poll<Result<(), E>>` instead of `Poll<()>`. This allows the
/// closure to surface errors happened in polling.
///
/// # Examples
///
/// ```
/// # #[tokio::main]
/// # async fn main() {
/// use std::task::Poll;
///
/// use fbthrift_util::poll_with_lock;
/// use tokio::sync::Mutex;
///
/// struct Foobar(i32);
/// let lock = Mutex::new(Foobar(132));
///
/// let locked_future =
///     poll_with_lock(&lock, |_locked, _ctx| Poll::Ready(Result::<(), ()>::Ok(())));
///
/// assert_eq!(locked_future.await.unwrap().0, 132);
/// # }
/// ```
pub fn poll_with_lock<'a, T, F, E>(lock: &'a Mutex<T>, f: F) -> PollWithLock<'a, T, F, E>
where
    F: FnMut(&mut MutexGuard<'a, T>, &mut Context<'_>) -> Poll<Result<(), E>> + Unpin,
{
    PollWithLock { lock, f }
}

impl<'a, T, F, E> Future for PollWithLock<'a, T, F, E>
where
    F: FnMut(&mut MutexGuard<'a, T>, &mut Context<'_>) -> Poll<Result<(), E>> + Unpin,
{
    type Output = Result<MutexGuard<'a, T>, E>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut fut = Box::pin(self.lock.lock());
        match fut.as_mut().poll(ctx) {
            Poll::Ready(mut locked) => match (self.f)(&mut locked, ctx) {
                Poll::Ready(Ok(())) => Poll::Ready(Ok(locked)),
                Poll::Ready(Err(e)) => Poll::Ready(Err(e)),
                Poll::Pending => Poll::Pending,
            },
            Poll::Pending => Poll::Pending,
        }
    }
}
