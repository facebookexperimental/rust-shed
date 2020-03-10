/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

//! This crate provides a client for accessing Memcache. The version on GitHub
//! is no-op for now.

#![deny(warnings, missing_docs, clippy::all, intra_doc_link_resolution_failure)]

mod client;
mod keygen;

pub use crate::client::{MemcacheClient, MemcacheGetType, MemcacheSetType};
pub use crate::keygen::KeyGen;

/// Memcache max size for key + value + overhead is around 1MB, so we are leaving 1KB for key +
/// overhead
pub const MEMCACHE_VALUE_MAX_SIZE: usize = 999_000;
