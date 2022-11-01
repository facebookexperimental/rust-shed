/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::fmt::Display;

use futures::Future;
use futures::Poll;

/// "Context" support for futures.
pub trait FutureErrorContext: Future + Sized {
    /// Add context to the error returned by this future
    fn context<D>(self, context: D) -> ContextFut<Self, D>
    where
        D: Display + Send + Sync + 'static;

    /// Add context created by provided function to the error returned by this future
    fn with_context<D, F>(self, f: F) -> WithContextFut<Self, F>
    where
        D: Display + Send + Sync + 'static,
        F: FnOnce() -> D;
}

impl<F, E> FutureErrorContext for F
where
    F: Future<Error = E> + Sized,
    E: Into<anyhow::Error>,
{
    fn context<D>(self, displayable: D) -> ContextFut<Self, D>
    where
        D: Display + Send + Sync + 'static,
    {
        ContextFut::new(self, displayable)
    }

    fn with_context<D, O>(self, f: O) -> WithContextFut<Self, O>
    where
        D: Display + Send + Sync + 'static,
        O: FnOnce() -> D,
    {
        WithContextFut::new(self, f)
    }
}

pub struct ContextFut<A, D> {
    inner: A,
    displayable: Option<D>,
}

impl<A, D> ContextFut<A, D> {
    pub fn new(future: A, displayable: D) -> Self {
        Self {
            inner: future,
            displayable: Some(displayable),
        }
    }
}

impl<A, E, D> Future for ContextFut<A, D>
where
    A: Future<Error = E>,
    E: Into<anyhow::Error>,
    D: Display + Send + Sync + 'static,
{
    type Item = A::Item;
    type Error = anyhow::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.inner.poll() {
            Err(err) => Err(err.into().context(
                self.displayable
                    .take()
                    .expect("poll called after future completion"),
            )),
            Ok(item) => Ok(item),
        }
    }
}

pub struct WithContextFut<A, F> {
    inner: A,
    displayable: Option<F>,
}

impl<A, F> WithContextFut<A, F> {
    pub fn new(future: A, displayable: F) -> Self {
        Self {
            inner: future,
            displayable: Some(displayable),
        }
    }
}

impl<A, E, F, D> Future for WithContextFut<A, F>
where
    A: Future<Error = E>,
    E: Into<anyhow::Error>,
    D: Display + Send + Sync + 'static,
    F: FnOnce() -> D,
{
    type Item = A::Item;
    type Error = anyhow::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.inner.poll() {
            Err(err) => {
                let f = self
                    .displayable
                    .take()
                    .expect("poll called after future completion");

                let context = f();
                Err(err.into().context(context))
            }
            Ok(item) => Ok(item),
        }
    }
}

#[cfg(test)]
mod test {
    use anyhow::format_err;
    use futures::future::err;

    use super::*;

    #[test]
    #[should_panic]
    fn poll_after_completion_fail() {
        let err = err::<(), _>(format_err!("foo").context("bar"));
        let mut err = err.context("baz");
        let _ = err.poll();
        let _ = err.poll();
    }

    #[test]
    #[should_panic]
    fn poll_after_completion_fail_with_context() {
        let err = err::<(), _>(format_err!("foo").context("bar"));
        let mut err = err.with_context(|| "baz");
        let _ = err.poll();
        let _ = err.poll();
    }

    #[test]
    #[should_panic]
    fn poll_after_completion_error() {
        let err = err::<(), _>(format_err!("foo"));
        let mut err = err.context("baz");
        let _ = err.poll();
        let _ = err.poll();
    }

    #[test]
    #[should_panic]
    fn poll_after_completion_error_with_context() {
        let err = err::<(), _>(format_err!("foo"));
        let mut err = err.with_context(|| "baz");
        let _ = err.poll();
        let _ = err.poll();
    }
}
