/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! This crate provides a client for accessing Memcache. The version on GitHub
//! is no-op for now.

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]

mod client;
mod keygen;

use anyhow::Result;

pub use crate::client::MemcacheClient;
pub use crate::client::MemcacheGetType;
pub use crate::client::MemcacheSetType;
pub use crate::keygen::KeyGen;

/// Memcache max size for key + value + overhead is around 1MB, so we are leaving 1KB for key +
/// overhead
pub const MEMCACHE_VALUE_MAX_SIZE: usize = 999_000;

/// Set the number of threads used for the memcache proxy.
pub fn set_proxy_threads_count(_count: usize) -> Result<()> {
    Ok(())
}

/// Set the maximum number of outstanding memcache requests that are
/// permitted.
pub fn set_max_outstanding_requests(_count: usize) -> Result<()> {
    Ok(())
}
