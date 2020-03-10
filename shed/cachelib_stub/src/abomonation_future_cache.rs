/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use anyhow::{Error, Result};
use futures_ext::BoxFuture;

use crate::lrucache::VolatileLruCachePool;

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
    _cache_key: &String,
    _entry: &T,
) -> Result<bool>
where
    T: abomonation::Abomonation + Clone + Send + 'static,
{
    Ok(false)
}
