/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! This crate provides a client for accessing Memcache. The version on GitHub
//! is no-op for now.

#[cfg(fbcode_build)]
pub use fb_memcache::*;
#[cfg(fbcode_build)]
use memcache_common as _; // used in oss
#[cfg(not(fbcode_build))]
pub use memcache_common::*;
