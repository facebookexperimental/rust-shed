/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use bytes::BufMut;
use bytes::Bytes;
use bytes::BytesMut;

/// Wrapper for using `Bytes` in `futures::TryStreamExt::try_collect` which
/// requires a trait bound `T: Default + Extend<Self::Ok>`. With this wrapper
/// we get to `try_collect` a stream of `Bytes` such as produced by Hyper
/// clients.
///
/// More explicitly, if `resp` is a `hyper::Response<hyper::Body>` then we write:
/// ```ignore
///     resp.into_body().try_collect::<BytesCollect>().into()
/// ```
/// to get back `Bytes`.
#[derive(Default)]
pub struct BytesCollect {
    buffer: BytesMut,
}

impl BytesCollect {
    /// Create default instance of BytesCollect
    pub fn new() -> Self {
        Self::default()
    }
}

impl Extend<Bytes> for BytesCollect {
    fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Bytes>,
    {
        for bytes in iter {
            self.buffer.put(bytes);
        }
    }
}

impl From<BytesCollect> for Bytes {
    fn from(collect: BytesCollect) -> Self {
        collect.buffer.freeze()
    }
}
