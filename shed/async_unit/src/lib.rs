/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, broken_intra_doc_links)]

//! Provides tokio_unit_test helper function for async unit tests that use Tokio

use futures_preview::future::Future;

/// Helper function for async unit tests - run the given code within a Tokio context, including
/// catching and reporting panics.
pub fn tokio_unit_test<F, V>(fut: F) -> V
where
    F: Future<Output = V>,
{
    let mut rt = tokio_compat::runtime::Runtime::new().unwrap();
    rt.block_on_std(fut)
}

#[cfg(test)]
mod test {
    use super::*;
    use futures::future::{self};

    #[test]
    fn nil_async() {
        tokio_unit_test(async {})
    }

    #[test]
    fn value_async() {
        let v = tokio_unit_test(async { 123 });
        assert_eq!(v, 123);
    }

    #[test]
    #[should_panic(expected = "Something went wrong")]
    fn panic_value_async() {
        async fn test() -> i32 {
            panic!("Something went wrong")
        }
        let v = tokio_unit_test(async { test().await });
        assert_eq!(v, 123);
    }

    #[test]
    #[should_panic(expected = "Ow! Something went wrong!")]
    fn panic_async() {
        tokio_unit_test(async { panic!("Ow! Something went wrong!") })
    }

    #[test]
    fn spawn_async() {
        tokio_unit_test(async {
            let _ = tokio_old::spawn(future::lazy(|| Ok(())));
        })
    }

    #[test]
    fn spawn_async_new_tokio() {
        tokio_unit_test(async {
            let _ = tokio::spawn(async move { () });
        })
    }
}
