/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

//! This crate provides a client for accessing Memcache. The version on GitHub
//! is no-op for now.

#[cfg(fbcode_build)]
pub use memcache::*;
#[cfg(not(fbcode_build))]
pub use memcache_common::*;
