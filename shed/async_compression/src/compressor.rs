/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Non-blocking, buffered compression.

use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::io;
use std::io::Write;
use std::result;

use bzip2::write::BzEncoder;
use flate2::write::GzEncoder;
use futures::Poll;
use tokio_io::AsyncWrite;

use crate::decompressor::DecompressorType;
use crate::raw::AsyncZstdEncoder;
use crate::raw::RawEncoder;
use crate::retry::retry_write;

/// Defines the supported compression types
#[derive(Clone, Copy, Debug)]
pub enum CompressorType {
    /// The [bzip2] compression with configs
    Bzip2(bzip2::Compression),
    /// The [flate2] compression with configs
    Gzip(flate2::Compression),
    /// The [zstd] compression
    Zstd {
        /// compression level, see [zstd::Encoder::new]
        level: i32,
    },
}

impl CompressorType {
    /// Returns the matching decompression type for this compression type
    pub fn decompressor_type(self) -> DecompressorType {
        match self {
            CompressorType::Bzip2(_) => DecompressorType::Bzip2,
            CompressorType::Gzip(_) => DecompressorType::Gzip,
            CompressorType::Zstd { .. } => DecompressorType::OverreadingZstd,
        }
    }
}

/// A wrapper around various compression libraries that compresses the data
/// passed to it via the [Write] trait invocations and writes it further to the
/// provided writer. It implements [AsyncWrite].
pub struct Compressor<W>
where
    W: AsyncWrite + 'static,
{
    c_type: CompressorType,
    inner: Box<dyn RawEncoder<W> + Send>,
}

impl<W> Compressor<W>
where
    W: AsyncWrite + Send + 'static,
{
    /// Creates and instance of [Compressor] that will use the provided
    /// [CompressorType] for compression and pass the result to the provided
    /// [Write]r
    pub fn new(w: W, ct: CompressorType) -> Self {
        Compressor {
            c_type: ct,
            inner: match ct {
                CompressorType::Bzip2(level) => Box::new(BzEncoder::new(w, level)),
                CompressorType::Gzip(level) => Box::new(GzEncoder::new(w, level)),
                CompressorType::Zstd { level } => Box::new(AsyncZstdEncoder::new(w, level)),
            },
        }
    }

    /// You need to finish the stream when you're done writing. This method
    /// calls the inner compression instance to finish the compression in their
    /// own way so that it might write the final data to inner [Write]r
    pub fn try_finish(self) -> result::Result<W, (Self, io::Error)> {
        match self.inner.try_finish() {
            Ok(writer) => Ok(writer),
            Err((encoder, e)) => Err((
                Compressor {
                    c_type: self.c_type,
                    inner: encoder,
                },
                e,
            )),
        }
    }
}

impl<W> Write for Compressor<W>
where
    W: AsyncWrite + Send,
{
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        retry_write(self.inner.by_ref(), buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W> AsyncWrite for Compressor<W>
where
    W: AsyncWrite + Send,
{
    #[inline]
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.inner.shutdown()
    }
}

impl<W: AsyncWrite> Debug for Compressor<W> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Compressor")
            .field("c_type", &self.c_type)
            .finish()
    }
}

/// Ensure that compressors implement Send.
fn _assert_send() {
    use std::io::Cursor;

    fn _assert<T: Send>(_val: T) {}

    _assert(Compressor::new(
        Cursor::new(Vec::new()),
        CompressorType::Bzip2(bzip2::Compression::Default),
    ));
}
