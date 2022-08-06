/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::io::BufRead;
use std::io::Read;
use std::io::Result;

use tokio_io::AsyncRead;

// This module provides an AsyncRead implementation that operates over a limited buffer size. This
// is particularly useful in combination with Framed I/O. Indeed, Framed accumulates data into a
// bufffer which is resized exponentially, but presents the full buffer to the underlying
// AsyncRead. If the AsyncRead zeros out the whole buffer prior to reading into it (which most do
// as that's the default behavior) but doesn't fill the whole buffer (which is likely because we
// can't read unlimited amounts of data at once), then we get quadratic behavior.
//
// For example, in Mononoke we might try to read 2 GiB of zstd-encoded data, and every time we call
// read(), that actually reads 128K of data into the buffer.
//
// The buffer resizes by doubling in size, so at best we'll at some point have a buffer that is 2
// GiB wide, with 1 GiB of data in it, and 1 GiB of data left to read.  This means we'll have to
// roundtrip through zstd 2048 times to finish filling the buffer.  Each of those roundtrips will
// start by zero-ing out the remaining space in the buffer, which will average 0.5 GiB, which means
// we'll have a total of 1024 GiB of data to zero-out.
//
// To make things worse, in Framed, the buffer sticks around after a successful decoding, so if we
// ever read a 2GB blob, then we have to zero out 2GB every time we read from the buffer in the
// future.

const DEFAULT_SIZE: usize = 8 * 1024;

#[derive(Debug)]
pub struct LimitedAsyncRead<R> {
    size: usize,
    inner: R,
}

impl<R> LimitedAsyncRead<R> {
    pub fn new(inner: R) -> Self {
        Self::new_with_size(inner, DEFAULT_SIZE)
    }

    pub fn new_with_size(inner: R, size: usize) -> Self {
        Self { inner, size }
    }

    #[inline]
    fn buf<'a>(&self, buf: &'a mut [u8]) -> &'a mut [u8] {
        if buf.len() <= self.size {
            buf
        } else {
            &mut buf[0..self.size]
        }
    }

    /// Get a reference to the inner AsyncRead.
    #[inline]
    pub fn get_ref(&self) -> &R {
        &self.inner
    }

    /// Get a mutable reference to the inner AsyncRead.
    #[inline]
    pub fn get_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    /// Returns the inner AsyncRead, consuming this structure.
    #[inline]
    pub fn into_inner(self) -> R {
        self.inner
    }
}

impl<R: Read> Read for LimitedAsyncRead<R> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.inner.read(self.buf(buf))
    }
}

impl<R: AsyncRead + Read> AsyncRead for LimitedAsyncRead<R> {
    #[inline]
    unsafe fn prepare_uninitialized_buffer(&self, buf: &mut [u8]) -> bool {
        self.inner.prepare_uninitialized_buffer(self.buf(buf))
    }
}

impl<R: BufRead> BufRead for LimitedAsyncRead<R> {
    #[inline]
    fn fill_buf(&mut self) -> Result<&[u8]> {
        self.inner.fill_buf()
    }

    #[inline]
    fn consume(&mut self, amt: usize) {
        self.inner.consume(amt)
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use futures::Async;

    use super::*;

    #[test]
    fn test_read_limited() {
        let b1 = [1, 1, 1, 1];
        let c = Cursor::new(&b1);
        let mut r = LimitedAsyncRead::new_with_size(c, 2);

        let mut b2 = [0, 0, 0, 0];
        let s = r.read(&mut b2).unwrap();
        assert_eq!(s, 2);
        assert_eq!(&b2, &[1, 1, 0, 0]);
    }

    #[test]
    fn test_read_not_limited() {
        let b1 = [1, 1, 1, 1];
        let c = Cursor::new(&b1);
        let mut r = LimitedAsyncRead::new_with_size(c, 10);

        let mut b2 = [0, 0, 0, 0];
        let s = r.read(&mut b2).unwrap();
        assert_eq!(s, 4);
        assert_eq!(&b2, &[1, 1, 1, 1]);
    }

    #[test]
    fn test_async_read_limited() {
        let b1 = [1, 1, 1, 1];
        let c = Cursor::new(&b1);
        let mut r = LimitedAsyncRead::new_with_size(c, 2);

        let mut b2 = [0, 0, 0, 0];
        let s = r.poll_read(&mut b2).unwrap();
        assert_eq!(s, Async::Ready(2));
        assert_eq!(&b2, &[1, 1, 0, 0]);
    }

    #[test]
    fn test_async_read_not_limited() {
        let b1 = [1, 1, 1, 1];
        let c = Cursor::new(&b1);
        let mut r = LimitedAsyncRead::new_with_size(c, 10);

        let mut b2 = [0, 0, 0, 0];
        let s = r.poll_read(&mut b2).unwrap();
        assert_eq!(s, Async::Ready(4));
        assert_eq!(&b2, &[1, 1, 1, 1]);
    }
}
