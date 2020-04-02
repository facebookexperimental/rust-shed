/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::Result;
use bytes::Bytes;
use fbinit::FacebookInit;
use futures::{future::ok, Future};
use std::time::Duration;

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
    pub fn get<K>(
        &self,
        _key: K,
    ) -> impl Future<Item = Option<MemcacheGetType>, Error = ()> + 'static
    where
        K: AsRef<str>,
    {
        ok(None)
    }

    /// Sets the Memcache value under `key` to `val`
    pub fn set<K, V>(&self, _key: K, _val: V) -> impl Future<Item = (), Error = ()> + 'static
    where
        K: AsRef<str>,
        MemcacheSetType: From<V>,
    {
        ok(())
    }

    /// Sets the Memcache value under `key` to `val` with the given expiration
    pub fn set_with_ttl<K, V>(
        &self,
        _key: K,
        _val: V,
        _exp: Duration,
    ) -> impl Future<Item = (), Error = ()> + 'static
    where
        K: AsRef<str>,
        MemcacheSetType: From<V>,
    {
        ok(())
    }

    /// Similar to `set`, but if the value is already present in Memcache it won't overwrite it.
    /// A boolean value is returned to say if the write was successful (true) or if a value was
    /// already present (false)
    pub fn add<K, V>(&self, _key: K, _val: V) -> impl Future<Item = bool, Error = ()> + 'static
    where
        K: AsRef<str>,
        MemcacheSetType: From<V>,
    {
        ok(true)
    }

    /// `add` equivalent of the `set_with_ttl` method
    pub fn add_with_ttl<K, V>(
        &self,
        _key: K,
        _val: V,
        _exp: Duration,
    ) -> impl Future<Item = bool, Error = ()> + 'static
    where
        K: AsRef<str>,
        MemcacheSetType: From<V>,
    {
        ok(true)
    }

    /// Removes the value under `key`.
    pub fn del<K>(&self, _key: K) -> impl Future<Item = (), Error = ()> + 'static
    where
        K: AsRef<str>,
    {
        ok(())
    }
}
