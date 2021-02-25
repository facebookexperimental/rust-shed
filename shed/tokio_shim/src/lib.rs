/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::Error;
use futures::{future::Future, ready, stream::Stream};
use pin_project::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

pub mod task {
    use super::*;

    #[pin_project(project = JoinHandleProj)]
    pub enum JoinHandle<T> {
        Tokio02(#[pin] tokio_02::task::JoinHandle<T>),
        Tokio10(#[pin] tokio_10::task::JoinHandle<T>),
    }

    impl<T> Future for JoinHandle<T>
    where
        T: Send + 'static,
    {
        type Output = Result<T, Error>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let ret = match self.project() {
                JoinHandleProj::Tokio02(f) => ready!(f.poll(cx)).map_err(Error::from),
                JoinHandleProj::Tokio10(f) => ready!(f.poll(cx)).map_err(Error::from),
            };

            Poll::Ready(ret)
        }
    }

    pub fn spawn<F>(fut: F) -> JoinHandle<<F as Future>::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        if let Ok(handle) = tokio_02::runtime::Handle::try_current() {
            return JoinHandle::Tokio02(handle.spawn(fut));
        }

        if let Ok(handle) = tokio_10::runtime::Handle::try_current() {
            return JoinHandle::Tokio10(handle.spawn(fut));
        }

        // This is what tokio::spawn would give you, so we don't try to do better here.
        panic!("A Tokio 0.2 or 1.0 runtime is required, but neither was running");
    }
}

pub mod time {
    use super::*;

    #[pin_project(project = SleepProj)]
    pub enum Sleep {
        Tokio02(#[pin] tokio_02::time::Delay),
        Tokio10(#[pin] tokio_10::time::Sleep),
    }

    impl Future for Sleep {
        type Output = ();

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            match self.project() {
                SleepProj::Tokio02(f) => f.poll(cx),
                SleepProj::Tokio10(f) => f.poll(cx),
            }
        }
    }

    pub fn sleep(duration: Duration) -> Sleep {
        if tokio_02::runtime::Handle::try_current().is_ok() {
            return Sleep::Tokio02(tokio_02::time::delay_for(duration));
        }

        if tokio_10::runtime::Handle::try_current().is_ok() {
            return Sleep::Tokio10(tokio_10::time::sleep(duration));
        }

        // This is what tokio::time::sleep would do.
        panic!("A Tokio 0.2 or 1.0 runtime is required, but neither was running");
    }

    #[pin_project(project = IntervalStreamProj)]
    pub enum IntervalStream {
        Tokio02(#[pin] tokio_02::time::Interval),
        Tokio10(#[pin] tokio_10_stream::wrappers::IntervalStream),
    }

    impl Stream for IntervalStream {
        type Item = Instant;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let ret = match self.project() {
                IntervalStreamProj::Tokio02(f) => ready!(f.poll_next(cx)).map(|i| i.into_std()),
                IntervalStreamProj::Tokio10(f) => ready!(f.poll_next(cx)).map(|i| i.into_std()),
            };

            Poll::Ready(ret)
        }
    }

    pub fn interval_stream(period: Duration) -> IntervalStream {
        if tokio_02::runtime::Handle::try_current().is_ok() {
            return IntervalStream::Tokio02(tokio_02::time::interval(period));
        }

        if tokio_10::runtime::Handle::try_current().is_ok() {
            let interval = tokio_10::time::interval(period);
            let stream = tokio_10_stream::wrappers::IntervalStream::new(interval);
            return IntervalStream::Tokio10(stream);
        }

        // This is what tokio::time::interval_at would do.
        panic!("A Tokio 0.2 or 1.0 runtime is required, but neither was running");
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use futures::{future, stream::StreamExt};

    #[test]
    fn test_02() -> Result<(), Error> {
        let mut rt = tokio_02::runtime::Builder::new()
            .enable_all()
            .basic_scheduler()
            .build()?;

        rt.block_on(async { task::spawn(future::ready(())).await })?;
        rt.block_on(async {
            time::sleep(Duration::from_millis(1)).await;
        });
        rt.block_on(async {
            time::interval_stream(Duration::from_millis(1)).next().await;
        });


        Ok(())
    }

    #[test]
    fn test_10() -> Result<(), Error> {
        let rt = tokio_10::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        rt.block_on(async { task::spawn(future::ready(())).await })?;
        rt.block_on(async {
            time::sleep(Duration::from_millis(1)).await;
        });
        rt.block_on(async { time::interval_stream(Duration::from_millis(1)).next().await });

        Ok(())
    }
}
