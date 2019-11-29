/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, intra_doc_link_resolution_failure)]

//! Provides tokio_unit_test helper function for async unit tests that use Tokio

use std::panic::{catch_unwind, UnwindSafe};
use std::sync::mpsc;

use tokio::prelude::*;

/// Helper function for async unit tests - run the given code within a Tokio context, including
/// catching and reporting panics.
pub fn tokio_unit_test<F, V>(func: F) -> V
where
    F: FnOnce() -> V + UnwindSafe + Send + 'static,
    V: Send + 'static,
{
    let (tx, rx) = mpsc::channel();

    tokio::run(future::lazy(move || {
        let ret = catch_unwind(func);
        tx.send(ret).expect("channel send failed");
        Ok(())
    }));

    match rx.recv().expect("channel recv failed") {
        Ok(v) => v,
        Err(err) => {
            if let Some(err_msg) = err.downcast_ref::<&str>() {
                panic!("Test failed: {}", err_msg);
            }
            if let Some(err_msg) = err.downcast_ref::<String>() {
                panic!("Test failed: {}", err_msg);
            }
            panic!("Test failed, cannot get full error message!");
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Duration;

    #[test]
    fn nil_async() {
        tokio_unit_test(|| {})
    }

    #[test]
    fn value_async() {
        let v = tokio_unit_test(|| 123);
        assert_eq!(v, 123);
    }

    #[test]
    #[should_panic(expected = "Something went wrong")]
    fn panic_value_async() {
        let v = tokio_unit_test(|| -> i32 { panic!("Something went wrong") });
        assert_eq!(v, 123);
    }

    #[test]
    #[should_panic(expected = "Something went wrong")]
    fn panic_async() {
        tokio_unit_test(|| panic!("Ow! Something went wrong!"))
    }

    #[test]
    fn spawn_async() {
        tokio_unit_test(|| {
            let _ = tokio::spawn(future::lazy(|| Ok(())));
        })
    }

    #[test]
    #[ignore] // can't handle panic/unwind in a spawned future
    #[should_panic(expected = "Something went wrong")]
    fn panic_spawn_async() {
        tokio_unit_test(|| {
            let _ = tokio::spawn(future::lazy(|| -> Result<(), ()> {
                panic!("Something went wrong");
            }));
        })
    }

    #[test]
    #[ignore] // Test failed: nested block_on: EnterError { reason: "attempted to run an executor while another executor is already running" }
    fn timer_async() {
        tokio_unit_test(move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(
                    tokio_timer::sleep(Duration::from_millis(100))
                        .map_err(|err| {
                            panic!("Timer error: {:?}", err);
                        })
                        .map(|_| {
                            println!("woke up");
                        }),
                )
                .expect("timer failed")
        })
    }
}
