/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use futures::Future;

pub fn tokio_test<F>(f: F) -> <F as Future>::Output
where
    F: Future,
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(f)
}

pub fn tokio_main<F>(tokio_workers: usize, f: F) -> <F as Future>::Output
where
    F: Future,
{
    // Trying to make this an Option<usize> is complicated :/
    if tokio_workers != 0 {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(tokio_workers)
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    } else {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(f)
    }
}
