/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::time::Duration;

use anyhow::Result;

use super::lrucache::VolatileLruCachePool;

pub fn get_cached<T>(_cache_pool: &VolatileLruCachePool, _cache_key: &String) -> Result<Option<T>> {
    Ok(None)
}

/// Returns `false` if the entry could not be inserted (e.g. another entry with the same
/// key was inserted first)
pub fn set_cached<T>(
    _cache_pool: &VolatileLruCachePool,
    _cache_key: &str,
    _entry: &T,
    _ttl: Option<Duration>,
) -> Result<bool> {
    Ok(false)
}

/// Encodes an item via bincode, and then inserts it into a volatile pool, replacing any pre-existing data.
pub fn set_or_replace_cached<T>(
    _cache_pool: &VolatileLruCachePool,
    _cache_key: &str,
    _entry: &T,
    _ttl: Option<Duration>,
) -> Result<()> {
    Ok(())
}
