/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use bytes::{Buf as BufNew, Bytes as BytesNew};
use bytes_old::{Buf as BufOld, Bytes as BytesOld};
use ref_cast::RefCast;

// Wrapper for using each of bytes 0.4 and 0.5's Bytes types as an
// implementation of the other version's Buf trait.
#[derive(RefCast)]
#[repr(transparent)]
pub struct BytesCompat<B> {
    pub inner: B,
}

impl<B> BytesCompat<B> {
    pub fn new(inner: B) -> Self {
        BytesCompat { inner }
    }
}

impl BufOld for BytesCompat<BytesNew> {
    fn remaining(&self) -> usize {
        self.inner.remaining()
    }

    fn bytes(&self) -> &[u8] {
        self.inner.bytes()
    }

    fn advance(&mut self, cnt: usize) {
        self.inner.advance(cnt)
    }
}

impl BufNew for BytesCompat<BytesOld> {
    fn remaining(&self) -> usize {
        self.inner.len()
    }

    fn bytes(&self) -> &[u8] {
        self.inner.as_ref()
    }

    fn advance(&mut self, cnt: usize) {
        self.inner.advance(cnt)
    }
}
