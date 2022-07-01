/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::future::FutureExt;
use futures::future::Shared;
use once_cell::sync::OnceCell;

/// A lazily-initialized shared future
///
/// Like `futures::future::Shared`, except it can be lazily initialized,
/// potentially with a readily available value.  This is useful when the
/// future might not be awaited on at all, or when sometimes the value is
/// already known at construction time.  In both cases it prevents the
/// overhead of setting up the generator and boxing the shared future when it
/// is not needed.
#[derive(Clone)]
pub enum LazyShared<T>
where
    T: Clone,
{
    Ready(T),
    Lazy(Arc<OnceCell<Shared<Pin<Box<dyn Future<Output = T> + Send>>>>>),
}

impl<T> LazyShared<T>
where
    T: Clone,
{
    /// Initialize the lazy-shared future with a ready value.
    pub fn new_ready(value: T) -> Self {
        LazyShared::Ready(value)
    }

    /// Initialize the lazy-shared future with no value.
    pub fn new_empty() -> Self {
        LazyShared::Lazy(Arc::new(OnceCell::new()))
    }

    /// Initialize the lazy-shared future with a future.
    pub fn new_future(f: impl Future<Output = T> + Send + 'static) -> Self {
        let cell = OnceCell::new();
        cell.set(f.boxed().shared()).unwrap();
        LazyShared::Lazy(Arc::new(cell))
    }

    /// Get the value of the shared future, providing an initialization
    /// function for the shared future if it has not yet been initialized.
    pub async fn get_or_init<F, Fut>(&self, init: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T> + Send + 'static,
    {
        match self {
            LazyShared::Ready(value) => value.clone(),
            LazyShared::Lazy(cell) => {
                cell.get_or_init(move || init().boxed().shared())
                    .clone()
                    .await
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;

    #[tokio::test]
    async fn test_ready() {
        let count = Arc::new(AtomicUsize::new(0));
        let lazy = LazyShared::new_ready(1u32);
        let value = lazy
            .get_or_init(|| {
                let count = count.clone();
                async move {
                    count.fetch_add(1, Ordering::Relaxed);
                    2u32
                }
            })
            .await;
        // Ready value is used, not lazy value.
        assert_eq!(value, 1u32);
        // Initializer was not called.
        assert_eq!(count.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn test_lazy() {
        let count = Arc::new(AtomicUsize::new(0));
        let lazy = LazyShared::new_empty();
        let value = lazy
            .get_or_init(|| {
                let count = count.clone();
                async move {
                    count.fetch_add(1, Ordering::Relaxed);
                    2u32
                }
            })
            .await;
        // Lazy value is used.
        assert_eq!(value, 2u32);
        // Initializer was called once.
        assert_eq!(count.load(Ordering::Relaxed), 1);

        // Read it again.
        let value = lazy
            .get_or_init(|| {
                let count = count.clone();
                async move {
                    count.fetch_add(1, Ordering::Relaxed);
                    3u32
                }
            })
            .await;
        // Original initializer value is used.
        assert_eq!(value, 2u32);
        // Initializer was not called again.
        assert_eq!(count.load(Ordering::Relaxed), 1);
    }
}
