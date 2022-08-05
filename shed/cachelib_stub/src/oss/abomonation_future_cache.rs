/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::Error;
use anyhow::Result;
use futures_ext::BoxFuture;
use std::time::Duration;

use super::lrucache::VolatileLruCachePool;

pub fn get_cached_or_fill<T, F>(
    _cache_pool: &VolatileLruCachePool,
    _cache_key: String,
    fetch: F,
) -> BoxFuture<Option<T>, Error>
where
    T: abomonation::Abomonation + Clone + Send + 'static,
    F: FnOnce() -> BoxFuture<Option<T>, Error>,
{
    fetch()
}

pub fn get_cached<T>(_cache_pool: &VolatileLruCachePool, _cache_key: &String) -> Result<Option<T>>
where
    T: abomonation::Abomonation + Clone + Send + 'static,
{
    Ok(None)
}

/// Returns `false` if the entry could not be inserted (e.g. another entry with the same
/// key was inserted first)
pub fn set_cached<T>(
    _cache_pool: &VolatileLruCachePool,
    _cache_key: &str,
    _entry: &T,
    _ttl: Option<Duration>,
) -> Result<bool>
where
    T: abomonation::Abomonation + Clone + Send + 'static,
{
    Ok(false)
}
