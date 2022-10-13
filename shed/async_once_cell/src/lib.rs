/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Async Once Cell.
//!
//! This is an async version of the `once_cell` crate.  It provides
//! `AsyncOnceCell`, which is equivalent to `OnceCell`, except that the
//! `get_or_init` and `get_or_try_init` methods take a function that returns a
//! future, which is then awaited to find the value.
//!
//! Only one caller of `get_or_init`, `get_or_try_init`, or `set` will
//! successfully initialize the `AsyncOnceCell`; the others will wait for it
//! to be initialized and then return the initialized value.  This means the
//! `set` method is also `async`, however it will only wait if there is a
//! concurrent `get_or_init` or `get_or_try_init` that is in the process of
//! initializing the cell.

use std::cell::UnsafeCell;
use std::future::Future;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use tokio::sync::Mutex as AsyncMutex;

/// Cell that is initialized exactly once, and can be initialized
/// asynchronously.
#[derive(Debug)]
pub struct AsyncOnceCell<T> {
    mutex: AsyncMutex<()>,
    is_initialized: AtomicBool,
    value: UnsafeCell<Option<T>>,
}

unsafe impl<T: Sync + Send> Sync for AsyncOnceCell<T> {}
unsafe impl<T: Send> Send for AsyncOnceCell<T> {}

impl<T> AsyncOnceCell<T> {
    /// Construct a new, uninitialized `AsyncOnceCell`.
    pub const fn new() -> Self {
        Self {
            mutex: AsyncMutex::const_new(()),
            is_initialized: AtomicBool::new(false),
            value: UnsafeCell::new(None),
        }
    }

    /// Construct a new initialized `AsyncOnceCell`.
    pub const fn new_with(value: T) -> Self {
        Self {
            mutex: AsyncMutex::const_new(()),
            is_initialized: AtomicBool::new(true),
            value: UnsafeCell::new(Some(value)),
        }
    }

    /// Returns `true` if this `AsyncOnceCell` has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.is_initialized.load(Ordering::Acquire)
    }

    /// Returns a reference the current value of the `AsyncOnceCell`.
    ///
    /// SAFETY: The cell must be initialized.
    pub unsafe fn get_unchecked(&self) -> &T {
        debug_assert!(self.is_initialized());
        let slot = &*self.value.get();
        slot.as_ref().expect("should be initialized")
    }

    /// Returns a reference to the current value, or `None` if it is
    /// not yet initialized.
    ///
    /// Note that if `None` is returned, then the cell might be in the process
    /// of initializing.
    pub fn get(&self) -> Option<&T> {
        if self.is_initialized() {
            return unsafe { Some(self.get_unchecked()) };
        }
        None
    }

    /// Attempt to set the value of the cell.  Returns `Ok(())` if the value
    /// was successfully set.  Returns `Err(value)` if the cell is already
    /// initialized with another value.
    ///
    /// This method is async, but will only wait if there is a concurrent
    /// initialization in progress.
    pub async fn set(&self, value: T) -> Result<(), T> {
        if self.is_initialized() {
            return Err(value);
        }
        let _mutex = self.mutex.lock().await;
        if self.is_initialized() {
            return Err(value);
        }
        let slot = self.value.get();
        unsafe {
            // SAFETY: Access is protected by `_mutex`.
            debug_assert!((*slot).is_none());
            *slot = Some(value);
        }
        self.is_initialized.store(true, Ordering::Release);
        Ok(())
    }

    /// Get the value of the cell, or initialize it asynchronously if it is
    /// not yet initialized.
    ///
    /// If the cell is not initialized and is not being initialized, then the
    /// callback is called, and the future it returns is awaited to get the
    /// value for the cell.
    pub async fn get_or_init<F, Fut>(&self, f: F) -> &T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        if self.is_initialized() {
            return unsafe { self.get_unchecked() };
        }
        let _mutex = self.mutex.lock().await;
        if self.is_initialized() {
            return unsafe { self.get_unchecked() };
        }
        let value = f().await;
        let slot = self.value.get();
        unsafe {
            // SAFETY: Access is protected by `_mutex`.
            debug_assert!((*slot).is_none());
            *slot = Some(value);
        }
        self.is_initialized.store(true, Ordering::Release);
        unsafe { self.get_unchecked() }
    }

    /// Get the value of the cell, or initialize it asynchronously if it is
    /// not yet initialized.
    ///
    /// If the cell is not initialized and is not being initialized, then the
    /// callback is called, and the future it returns is awaited to get the
    /// value for the cell.
    ///
    /// If the future returns an error, then that error is returned from this
    /// method, and the cell is left uninitialized.
    pub async fn get_or_try_init<F, Fut, E>(&self, f: F) -> Result<&T, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        if self.is_initialized() {
            return unsafe { Ok(self.get_unchecked()) };
        }
        let _mutex = self.mutex.lock().await;
        if self.is_initialized() {
            return unsafe { Ok(self.get_unchecked()) };
        }
        let value = f().await?;
        let slot = self.value.get();
        unsafe {
            // SAFETY: Access is protected by `_mutex`.
            debug_assert!((*slot).is_none());
            *slot = Some(value);
        }
        self.is_initialized.store(true, Ordering::Release);
        unsafe { Ok(self.get_unchecked()) }
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;
    use std::time::Duration;

    use rand::thread_rng;
    use rand::Rng;

    use super::*;

    #[tokio::test]
    async fn new_with() {
        assert_eq!(AsyncOnceCell::new_with(123).get(), Some(&123));
        assert_eq!(
            AsyncOnceCell::new_with(456)
                .get_or_init(|| async { panic!("must not be called") })
                .await,
            &456
        );
    }

    #[tokio::test]
    async fn set_get() {
        let aoc = AsyncOnceCell::new();
        assert_eq!(aoc.get(), None);
        assert_eq!(aoc.set(123).await, Ok(()));
        assert_eq!(aoc.get(), Some(&123));
        assert_eq!(aoc.set(456).await, Err(456));
        assert_eq!(aoc.get(), Some(&123));
    }

    #[tokio::test]
    async fn get_or_init() {
        let aoc = AsyncOnceCell::new();
        assert_eq!(aoc.get(), None);
        assert_eq!(aoc.get_or_init(|| async { 123 }).await, &123);
        assert_eq!(aoc.get(), Some(&123));
        assert_eq!(aoc.get_or_init(|| async { 456 }).await, &123);
        assert_eq!(aoc.get(), Some(&123));
    }

    #[tokio::test]
    async fn get_or_try_init() {
        let aoc = AsyncOnceCell::new();
        assert_eq!(aoc.get(), None);
        assert_eq!(
            aoc.get_or_try_init(|| async { Err("error!") }).await,
            Err("error!")
        );
        assert_eq!(aoc.get(), None);
        assert_eq!(
            aoc.get_or_try_init(|| async { Ok::<_, ()>(123) }).await,
            Ok(&123)
        );
        assert_eq!(aoc.get(), Some(&123));
        assert_eq!(
            aoc.get_or_try_init(|| async { Ok::<_, ()>(456) }).await,
            Ok(&123)
        );
        assert_eq!(aoc.get(), Some(&123));
        assert_eq!(
            aoc.get_or_try_init(|| async { Err("error!") }).await,
            Ok(&123)
        );
        assert_eq!(aoc.get(), Some(&123));
    }

    #[tokio::test]
    async fn concurrent_get_or_init() {
        let aoc: AsyncOnceCell<i32> = AsyncOnceCell::new();
        let delay1 = Duration::from_millis(thread_rng().gen_range(0..100));
        let delay2 = Duration::from_millis(thread_rng().gen_range(0..100));
        let count = AtomicUsize::new(0);
        let f = futures::future::join(
            async {
                tokio::time::sleep(delay1).await;
                aoc.get_or_init(|| async {
                    tokio::time::sleep(delay1 + delay2).await;
                    count.fetch_add(1, Ordering::Relaxed);
                    123
                })
                .await
                .clone()
            },
            async {
                tokio::time::sleep(delay2).await;
                aoc.get_or_init(|| async {
                    tokio::time::sleep(delay1 + delay2).await;
                    count.fetch_add(1, Ordering::Relaxed);
                    456
                })
                .await
                .clone()
            },
        );
        let (v1, v2) = f.await;

        // The two values should be the same.
        assert_eq!(v1, v2);

        // They should be one of the two possibilities (depending on which one
        // executed).
        assert!(v1 == 123 || v1 == 456, "cell has unexpected value: {}", v1);

        // The underlying AsyncOnceCell should have that value.
        assert_eq!(aoc.get(), Some(&v1));

        // Only one future should have been executed.
        assert_eq!(count.load(Ordering::Relaxed), 1);
    }
}
