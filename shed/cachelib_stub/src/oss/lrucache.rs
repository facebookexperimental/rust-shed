/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::io;
use std::io::Cursor;
use std::io::Read;
use std::io::Write;
use std::marker::PhantomData;

use anyhow::Result;
use bytes::buf::UninitSlice;
use bytes::Buf;
use bytes::BufMut;
use bytes::Bytes;

pub fn init_cacheadmin() -> Result<()> {
    Ok(())
}

/// Get the remaining unallocated space in the cache
pub fn get_available_space() -> Result<usize> {
    Ok(0)
}

/// Obtain a new pool from the cache. Pools are sub-caches that have their own slice of the cache's
/// available memory, but that otherwise function as independent caches. You cannot write to a
/// cache without a pool. Note that pools are filled in slabs of 4 MiB, so the actual size you
/// receive is floor(pool_bytes / 4 MiB).
/// If the pool already exists, you will get the pre-existing pool instead of a new pool
pub fn get_or_create_pool(pool_name: &str, _pool_bytes: usize) -> Result<LruCachePool> {
    Ok(LruCachePool {
        pool_name: pool_name.to_owned(),
    })
}

/// Obtain a new volatile pool from the cache.
pub fn get_or_create_volatile_pool(
    pool_name: &str,
    _pool_bytes: usize,
) -> Result<VolatileLruCachePool> {
    Ok(VolatileLruCachePool {
        inner: LruCachePool {
            pool_name: pool_name.to_owned(),
        },
    })
}

/// Returns an existing cache pool by name. Returns Some(pool) if the pool exists, None if the
/// pool has not yet been created.
pub fn get_pool(_pool_name: &str) -> Option<LruCachePool> {
    None
}

/// Obtains an existing volatile cache pool by name.
pub fn get_volatile_pool(_pool_name: &str) -> Result<Option<VolatileLruCachePool>> {
    Ok(None)
}

/// A handle to data stored inside the cache. Can be used to get accessor structs
pub struct LruCacheHandle<T> {
    _marker: PhantomData<T>,
}

pub enum ReadOnly {}
pub enum ReadWrite {}
pub enum ReadWriteShared {}

impl<T> LruCacheHandle<T> {
    pub fn get_reader<'a>(&'a self) -> Result<LruCacheHandleReader<'a>> {
        Ok(LruCacheHandleReader {
            buffer: Cursor::new(Vec::new()),
            _phantom: PhantomData,
        })
    }
}

impl LruCacheHandle<ReadWrite> {
    pub fn get_writer<'a>(&'a mut self) -> Result<LruCacheHandleWriter<'a>> {
        Ok(LruCacheHandleWriter {
            buffer: Cursor::new(Vec::new()),
            _phantom: PhantomData,
        })
    }
}

impl LruCacheHandle<ReadWriteShared> {
    pub fn get_writer<'a>(&'a mut self) -> Result<LruCacheHandleWriter<'a>> {
        Ok(LruCacheHandleWriter {
            buffer: Cursor::new(Vec::new()),
            _phantom: PhantomData,
        })
    }

    pub fn get_remote_handle(&self) -> Result<LruCacheRemoteHandle<'_>> {
        Ok(LruCacheRemoteHandle {
            _phantom: PhantomData,
        })
    }
}

/// A read-only handle to an element in the cache. Implements io::Read and bytes::Buf
/// for easy access to the data within the handle
pub struct LruCacheHandleReader<'a> {
    buffer: Cursor<Vec<u8>>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> Buf for LruCacheHandleReader<'a> {
    fn remaining(&self) -> usize {
        self.buffer.remaining()
    }

    fn chunk(&self) -> &[u8] {
        Buf::chunk(&self.buffer)
    }

    fn advance(&mut self, cnt: usize) {
        self.buffer.advance(cnt)
    }
}

impl<'a> Read for LruCacheHandleReader<'a> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.buffer.read(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.buffer.read_exact(buf)
    }
}

/// A writable handle to an element in the cache. Implements io::{Read, Write} and
/// bytes::{Buf, BufMut} for easy access to the data within the handle
pub struct LruCacheHandleWriter<'a> {
    buffer: Cursor<Vec<u8>>,
    _phantom: PhantomData<&'a ()>,
}

/// SAFETY: Only calls to advance_mut modify the current position.
unsafe impl<'a> BufMut for LruCacheHandleWriter<'a> {
    #[inline]
    fn remaining_mut(&self) -> usize {
        let pos = self.buffer.position();
        let len = self.buffer.get_ref().len();
        len.saturating_sub(pos as usize)
    }

    #[inline]
    unsafe fn advance_mut(&mut self, cnt: usize) {
        self.buffer
            .set_position(self.buffer.position() + cnt as u64);
    }

    #[inline]
    fn chunk_mut(&mut self) -> &mut UninitSlice {
        let pos = self.buffer.position();
        let remaining = self
            .buffer
            .get_mut()
            .get_mut(pos as usize..)
            .unwrap_or(&mut []);

        unsafe { UninitSlice::from_raw_parts_mut(remaining.as_mut_ptr(), remaining.len()) }
    }
}

impl<'a> Write for LruCacheHandleWriter<'a> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.buffer.flush()
    }
}

/// A handle remotely access data stored inside the cache. Tied to the lifetime of the
/// LruCacheHandle it is created from.
pub struct LruCacheRemoteHandle<'a> {
    _phantom: PhantomData<&'a ()>,
}

impl<'a> LruCacheRemoteHandle<'a> {
    pub fn get_offset(&self) -> usize {
        0
    }

    pub fn get_length(&self) -> usize {
        0
    }
}

#[derive(Clone)]
pub struct LruCachePool {
    #[allow(dead_code)]
    pool_name: String,
}

impl LruCachePool {
    /// Allocate memory for a key of known size; this will claim the memory until the handle is
    /// dropped or inserted into the cache.
    /// Note that if you do not insert the handle, it will not be visible to `get`, and the
    /// associated memory will be pinned until the handle is inserted or dropped. Do not hold onto
    /// handles for long time periods, as this will reduce cachelib's efficiency.
    pub fn allocate<K>(
        &self,
        _key: K,
        _size: usize,
    ) -> Result<Option<LruCacheHandle<ReadWriteShared>>>
    where
        K: AsRef<[u8]>,
    {
        Ok(None)
    }

    /// Insert a previously allocated handle into the cache, making it visible to `get`
    /// Returns `false` if the handle could not be inserted (e.g. another handle with the same
    /// key was inserted first)
    pub fn insert_handle(&self, _handle: LruCacheHandle<ReadWriteShared>) -> Result<bool> {
        Ok(false)
    }

    /// Insert a key->value mapping into the pool. Returns true if the insertion was successful,
    /// false otherwise. This will not overwrite existing data.
    pub fn set<K, V>(&self, _key: K, _value: V) -> Result<bool>
    where
        K: AsRef<[u8]>,
        V: Buf,
    {
        Ok(false)
    }

    /// Insert a key->value mapping into the pool. Returns true if the insertion was successful,
    /// false otherwise. This will overwrite existing data.
    pub fn set_or_replace<K, V>(&self, _key: K, _value: V) -> Result<bool>
    where
        K: AsRef<[u8]>,
        V: Buf,
    {
        Ok(false)
    }

    /// Fetch a read handle for a key. Returns None if the key could not be found in the pool,
    /// Some(handle) if the key was found in the pool
    /// Note that the handle will stop the key being evicted from the cache until dropped -
    /// do not hold onto the handle for longer than the minimum necessary time.
    pub fn get_handle<K>(&self, _key: K) -> Result<Option<LruCacheHandle<ReadWriteShared>>>
    where
        K: AsRef<[u8]>,
    {
        Ok(None)
    }

    /// Fetch the value for a key. Returns None if the key could not be found in the pool,
    /// Some(value) if the key was found in the pool
    pub fn get<K>(&self, _key: K) -> Result<Option<Bytes>>
    where
        K: AsRef<[u8]>,
    {
        Ok(None)
    }

    /// Return the current size of this pool
    pub fn get_size(&self) -> Result<usize> {
        Ok(0)
    }

    /// Increase the size of the pool by size, returning true if it grew, false if there is
    /// insufficent available memory to grow this pool
    pub fn grow_pool(&self, _size: usize) -> Result<bool> {
        Ok(false)
    }

    /// Decrease the size of the pool by size, returning `true` if the pool will shrink, `false`
    /// if the pool is already smaller than size.
    /// Note that the actual shrinking is done asynchronously, based on the PoolResizeConfig
    /// supplied at the creation of the cachelib setup.
    pub fn shrink_pool(&self, _size: usize) -> Result<bool> {
        Ok(true)
    }

    /// Move bytes from this pool to another pool, returning true if this pool can shrink,
    /// false if you asked to move more bytes than are available
    /// Note that the actual shrinking of this pool is done asynchronously, based on the
    /// PoolResizeConfig supplied at the creation of the cachelib setup.
    pub fn transfer_capacity_to(&self, _dest: &Self, _bytes: usize) -> Result<bool> {
        Ok(false)
    }
}

#[derive(Clone)]
pub struct VolatileLruCachePool {
    #[allow(dead_code)]
    inner: LruCachePool,
}

impl VolatileLruCachePool {
    pub fn allocate<K>(&self, _key: K, _size: usize) -> Result<Option<LruCacheHandle<ReadWrite>>>
    where
        K: AsRef<[u8]>,
    {
        Ok(None)
    }

    pub fn insert_handle(&self, _handle: LruCacheHandle<ReadWrite>) -> Result<bool> {
        Ok(false)
    }

    pub fn set<K, V>(&self, _key: K, _value: V) -> Result<bool>
    where
        K: AsRef<[u8]>,
        V: Buf,
    {
        Ok(false)
    }

    pub fn set_or_replace<K, V>(&self, _key: K, _value: V) -> Result<bool>
    where
        K: AsRef<[u8]>,
        V: Buf,
    {
        Ok(false)
    }

    pub fn get_handle<K>(&self, _key: K) -> Result<Option<LruCacheHandle<ReadOnly>>>
    where
        K: AsRef<[u8]>,
    {
        Ok(None)
    }

    pub fn get<K>(&self, _key: K) -> Result<Option<Bytes>>
    where
        K: AsRef<[u8]>,
    {
        Ok(None)
    }

    pub fn get_size(&self) -> Result<usize> {
        Ok(0)
    }

    pub fn grow_pool(&self, _size: usize) -> Result<bool> {
        Ok(false)
    }

    pub fn shrink_pool(&self, _size: usize) -> Result<bool> {
        Ok(true)
    }

    pub fn transfer_capacity_to(&self, _dest: &Self, _bytes: usize) -> Result<bool> {
        Ok(false)
    }
}
