/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::time::Duration;

use anyhow::Result;
use bytes::Bytes;
use fbinit::FacebookInit;

/// Type of value returned from memcache
pub type MemcacheGetType = Vec<u8>;
/// Type of value that can be written to memcache
pub type MemcacheSetType = Bytes;

/// Client for Memcache
#[derive(Clone, Debug)]
pub struct MemcacheClient;

impl MemcacheClient {
    /// Return a new instance of MemcacheClient.
    pub fn new(_fb: FacebookInit) -> Result<Self> {
        Ok(MemcacheClient)
    }

    /// Gets the Memcache value under `key`
    pub async fn get<K>(&self, _key: K) -> Result<Option<MemcacheGetType>>
    where
        K: AsRef<str>,
    {
        Ok(None)
    }

    /// Sets the Memcache value under `key` to `val`
    pub async fn set<K, V>(&self, _key: K, _val: V) -> Result<()>
    where
        K: AsRef<str>,
        MemcacheSetType: From<V>,
    {
        Ok(())
    }

    /// Sets the Memcache value under `key` to `val` with the given expiration
    pub async fn set_with_ttl<K, V>(&self, _key: K, _val: V, _exp: Duration) -> Result<()>
    where
        K: AsRef<str>,
        MemcacheSetType: From<V>,
    {
        Ok(())
    }

    /// Similar to `set`, but if the value is already present in Memcache it won't overwrite it.
    /// A boolean value is returned to say if the write was successful (true) or if a value was
    /// already present (false)
    pub async fn add<K, V>(&self, _key: K, _val: V) -> Result<bool>
    where
        K: AsRef<str>,
        MemcacheSetType: From<V>,
    {
        Ok(true)
    }

    /// `add` equivalent of the `set_with_ttl` method
    pub async fn add_with_ttl<K, V>(&self, _key: K, _val: V, _exp: Duration) -> Result<bool>
    where
        K: AsRef<str>,
        MemcacheSetType: From<V>,
    {
        Ok(true)
    }

    /// Removes the value under `key`.
    pub async fn del<K>(&self, _key: K) -> Result<()>
    where
        K: AsRef<str>,
    {
        Ok(())
    }
}
