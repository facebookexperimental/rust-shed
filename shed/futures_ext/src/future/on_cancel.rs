/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::pin::Pin;

use futures::future::Future;
use futures::ready;
use futures::task::{Context, Poll};
use pin_project::{pin_project, pinned_drop};

/// Future combinator that executes the `on_cancel` closure if the inner future
/// is cancelled (dropped before completion).
#[pin_project(PinnedDrop)]
pub struct OnCancel<Fut, OnCancelFn>
where
    Fut: Future,
    OnCancelFn: FnOnce(),
{
    #[pin]
    inner: Fut,

    on_cancel: Option<OnCancelFn>,
}

impl<Fut, OnCancelFn> OnCancel<Fut, OnCancelFn>
where
    Fut: Future,
    OnCancelFn: FnOnce(),
{
    /// Construct an `OnCancel` combinator that will run `on_cancel` if `inner`
    /// is cancelled.
    pub fn new(inner: Fut, on_cancel: OnCancelFn) -> Self {
        Self {
            inner,
            on_cancel: Some(on_cancel),
        }
    }
}

impl<Fut, OnCancelFn> Future for OnCancel<Fut, OnCancelFn>
where
    Fut: Future,
    OnCancelFn: FnOnce(),
{
    type Output = Fut::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let v = ready!(this.inner.poll(cx));
        *this.on_cancel = None;
        Poll::Ready(v)
    }
}

#[pinned_drop]
impl<Fut, OnCancelFn> PinnedDrop for OnCancel<Fut, OnCancelFn>
where
    Fut: Future,
    OnCancelFn: FnOnce(),
{
    fn drop(self: Pin<&mut Self>) {
        let this = self.project();
        if let Some(on_cancel) = this.on_cancel.take() {
            on_cancel()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn runs_when_cancelled() {
        let cancelled = AtomicBool::new(false);
        let fut = OnCancel::new(async {}, || cancelled.store(true, Ordering::Relaxed));
        drop(fut);
        assert!(cancelled.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn doesnt_run_when_complete() {
        let cancelled = AtomicBool::new(false);
        let fut = OnCancel::new(async {}, || cancelled.store(true, Ordering::Relaxed));
        fut.await;
        assert!(!cancelled.load(Ordering::Relaxed));
    }
}
