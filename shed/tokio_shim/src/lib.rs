/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::Error;
use futures::{
    future::{BoxFuture, Future, FutureExt, TryFutureExt},
    stream::{Stream, StreamExt},
};
use std::time::{Duration, Instant};

pub mod task {
    use super::*;

    pub fn spawn<F>(fut: F) -> BoxFuture<'static, Result<F::Output, Error>>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        if let Ok(handle) = tokio_02::runtime::Handle::try_current() {
            return handle.spawn(fut).map_err(Into::into).boxed();
        }

        if let Ok(handle) = tokio_10::runtime::Handle::try_current() {
            return handle.spawn(fut).map_err(Into::into).boxed();
        }

        // This is what tokio::spawn would give you, so we don't try to do better here.
        panic!("A Tokio 0.2 or 1.0 runtime is required, but neither was running");
    }
}

pub mod time {
    use super::*;

    pub async fn sleep(duration: Duration) {
        if tokio_02::runtime::Handle::try_current().is_ok() {
            return tokio_02::time::delay_for(duration).await;
        }

        if tokio_10::runtime::Handle::try_current().is_ok() {
            return tokio_10::time::sleep(duration).await;
        }

        // This is what tokio::time::sleep would do (note that it panics when polled, not when
        // created).
        panic!("A Tokio 0.2 or 1.0 runtime is required, but neither was running");
    }

    pub fn interval_stream(period: Duration) -> impl Stream<Item = Instant> + 'static + Send {
        async move {
            if tokio_02::runtime::Handle::try_current().is_ok() {
                return tokio_02::time::interval(period)
                    .map(|i| i.into_std())
                    .boxed();
            }

            if tokio_10::runtime::Handle::try_current().is_ok() {
                let interval = tokio_10::time::interval(period);
                return tokio_10_stream::wrappers::IntervalStream::new(interval)
                    .map(|i| i.into_std())
                    .boxed();
            }

            // This is what tokio::time::interval_at would do (note that it panics when polled, not
            // when created).
            panic!("A Tokio 0.2 or 1.0 runtime is required, but neither was running");
        }
        .flatten_stream()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use futures::future;

    #[test]
    fn test_02() -> Result<(), Error> {
        let mut rt = tokio_02::runtime::Builder::new()
            .enable_all()
            .basic_scheduler()
            .build()?;

        rt.block_on(async { task::spawn(future::ready(())).await })?;
        rt.block_on(time::sleep(Duration::from_millis(1)));
        rt.block_on(
            time::interval_stream(Duration::from_millis(1))
                .boxed()
                .next(),
        );


        Ok(())
    }

    #[test]
    fn test_10() -> Result<(), Error> {
        let rt = tokio_10::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        rt.block_on(async { task::spawn(future::ready(())).await })?;
        rt.block_on(time::sleep(Duration::from_millis(1)));
        rt.block_on(
            time::interval_stream(Duration::from_millis(1))
                .boxed()
                .next(),
        );

        Ok(())
    }

    #[test]
    fn test_auto_traits() {
        fn assert_static_send<T: 'static + Send>(_: T) {}

        assert_static_send(time::sleep(Duration::from_millis(1)));
        assert_static_send(time::interval_stream(Duration::from_millis(1)));
    }
}
