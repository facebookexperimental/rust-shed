/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use bytes::Buf as BufNew;
use bytes::Bytes as BytesNew;
use bytes_old::Buf as BufOld;
use bytes_old::Bytes as BytesOld;
use ref_cast::RefCast;

/// Wrapper for using each of bytes 0.4 and 0.5's Bytes types as an
/// implementation of the other version's Buf trait.
#[derive(RefCast)]
#[repr(transparent)]
pub struct BytesCompat<B> {
    /// Inner instance of either bytes 0.4 or 0.5
    pub inner: B,
}

impl<B> BytesCompat<B> {
    /// Create a new BytesCompat wrapper around the provided instance
    pub fn new(inner: B) -> Self {
        BytesCompat { inner }
    }
}

impl BufOld for BytesCompat<BytesNew> {
    fn remaining(&self) -> usize {
        self.inner.remaining()
    }

    fn bytes(&self) -> &[u8] {
        self.inner.chunk()
    }

    fn advance(&mut self, cnt: usize) {
        self.inner.advance(cnt)
    }
}

impl BufNew for BytesCompat<BytesOld> {
    fn remaining(&self) -> usize {
        self.inner.len()
    }

    fn chunk(&self) -> &[u8] {
        self.inner.as_ref()
    }

    fn advance(&mut self, cnt: usize) {
        self.inner.advance(cnt)
    }
}
