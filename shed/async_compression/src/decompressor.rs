/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Non-blocking, buffered compression and decompression

use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;
use std::io;
use std::io::BufRead;
use std::io::Read;

use bzip2::bufread::BzDecoder;
use flate2::bufread::GzDecoder;
use tokio_io::AsyncRead;
use zstd::Decoder as ZstdDecoder;

use crate::raw::RawDecoder;

/// A wrapper around various compression libraries that decompresses from the
/// inner [Read]er and exposes a [Read] trait API of its own. It implements
/// [AsyncRead].
pub struct Decompressor<'a, R>
where
    R: AsyncRead + BufRead + 'a + Send,
{
    d_type: DecompressorType,
    inner: Box<dyn RawDecoder<R> + 'a + Send>,
}

/// Defines the supported decompression types
#[derive(Clone, Copy, Debug)]
pub enum DecompressorType {
    /// The [bzip2] decompression
    Bzip2,
    /// The [flate2] decompression
    Gzip,
    ///  The [zstd] decompression
    ///
    /// The Zstd Decoder is overreading it's input. Consider this situation: you have a Reader that
    /// returns parts of it's data compressed with Zstd and the remainder decompressed. Gzip and
    /// Bzip2 will consume only the compressed bytes leaving the remainder untouched. The Zstd
    /// though will consume some of the decomressed bytes, so that once you call `::into_inner()`
    /// on it, the returned Reader will not contain the decomressed bytes.
    ///
    /// Advice: use only if the entire Reader content needs to be decompressed
    /// You have been warned
    OverreadingZstd,
}

impl<'a, R> Decompressor<'a, R>
where
    R: AsyncRead + BufRead + 'a + Send,
{
    /// Creates and instance of [Decompressor] that will use the provided
    /// [DecompressorType] for decompression of data read from the provided
    /// [Read]er
    pub fn new(r: R, dt: DecompressorType) -> Self {
        Decompressor {
            d_type: dt,
            inner: match dt {
                DecompressorType::Bzip2 => Box::new(BzDecoder::new(r)),
                DecompressorType::Gzip => Box::new(GzDecoder::new(r)),
                DecompressorType::OverreadingZstd => Box::new(
                    ZstdDecoder::with_buffer(r).expect("ZstdDecoder failed to create. Are we OOM?"),
                ),
            },
        }
    }

    /// Get a reference to the inner decompressor
    #[inline]
    pub fn get_ref(&self) -> &R {
        self.inner.get_ref()
    }

    /// Get a mutable reference to the inner decompressor
    #[inline]
    pub fn get_mut(&mut self) -> &mut R {
        self.inner.get_mut()
    }

    /// Returns the inner decompressor consuming this structure
    #[inline]
    pub fn into_inner(self) -> R {
        self.inner.into_inner()
    }
}

impl<'a, R: AsyncRead + BufRead + 'a + Send> Read for Decompressor<'a, R> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<'a, R: AsyncRead + BufRead + 'a + Send> AsyncRead for Decompressor<'a, R> {}

impl<'a, R: AsyncRead + BufRead + 'a + Send> Debug for Decompressor<'a, R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Decompressor")
            .field("decoder_type", &self.d_type)
            .finish()
    }
}
