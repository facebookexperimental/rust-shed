/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Raw upstream decoders, plus a uniform interface for accessing them.

use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;
use std::io::Write;
use std::result;

use bzip2::bufread::BzDecoder;
use bzip2::write::BzEncoder;
use flate2::bufread::GzDecoder;
use flate2::write::GzEncoder;
use futures::Poll;
use tokio_io::AsyncWrite;
use zstd::Decoder as ZstdDecoder;
use zstd::Encoder as ZstdEncoder;

pub trait RawDecoder<R: BufRead>: Read {
    fn get_ref(&self) -> &R;
    fn get_mut(&mut self) -> &mut R;
    fn into_inner(self: Box<Self>) -> R;
}

impl<R: BufRead> RawDecoder<R> for BzDecoder<R> {
    #[inline]
    fn get_ref(&self) -> &R {
        BzDecoder::get_ref(self)
    }

    #[inline]
    fn get_mut(&mut self) -> &mut R {
        BzDecoder::get_mut(self)
    }

    #[inline]
    fn into_inner(self: Box<Self>) -> R {
        BzDecoder::into_inner(*self)
    }
}

impl<R: BufRead> RawDecoder<R> for GzDecoder<R> {
    #[inline]
    fn get_ref(&self) -> &R {
        GzDecoder::get_ref(self)
    }

    #[inline]
    fn get_mut(&mut self) -> &mut R {
        GzDecoder::get_mut(self)
    }

    #[inline]
    fn into_inner(self: Box<Self>) -> R {
        GzDecoder::into_inner(*self)
    }
}

impl<R: BufRead> RawDecoder<R> for ZstdDecoder<'_, BufReader<R>> {
    #[inline]
    fn get_ref(&self) -> &R {
        ZstdDecoder::get_ref(self).get_ref()
    }

    #[inline]
    fn get_mut(&mut self) -> &mut R {
        ZstdDecoder::get_mut(self).get_mut()
    }

    #[inline]
    fn into_inner(self: Box<Self>) -> R {
        ZstdDecoder::finish(*self).into_inner()
    }
}

pub trait RawEncoder<W>: AsyncWrite
where
    W: AsyncWrite + Send,
{
    fn try_finish(self: Box<Self>)
    -> result::Result<W, (Box<dyn RawEncoder<W> + Send>, io::Error)>;
}

impl<W> RawEncoder<W> for BzEncoder<W>
where
    W: AsyncWrite + Send + 'static,
{
    #[inline]
    fn try_finish(
        mut self: Box<Self>,
    ) -> result::Result<W, (Box<dyn RawEncoder<W> + Send>, io::Error)> {
        match BzEncoder::try_finish(&mut self) {
            Ok(()) => Ok(BzEncoder::finish(*self).unwrap()),
            Err(e) => Err((self, e)),
        }
    }
}

/// A wrapper around GzEncoder which depends on and implements AsyncWrite.
pub struct AsyncGzEncoder<W: AsyncWrite>(GzEncoder<W>);

impl<W: AsyncWrite> AsyncGzEncoder<W> {
    pub fn new(obj: W, level: flate2::Compression) -> Self {
        AsyncGzEncoder(GzEncoder::new(obj, level))
    }
}

impl<W: AsyncWrite> Write for AsyncGzEncoder<W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl<W: AsyncWrite> AsyncWrite for AsyncGzEncoder<W> {
    #[inline]
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.0.get_mut().shutdown()
    }
}

impl<W> RawEncoder<W> for AsyncGzEncoder<W>
where
    W: AsyncWrite + Send + 'static,
{
    fn try_finish(
        mut self: Box<Self>,
    ) -> result::Result<W, (Box<dyn RawEncoder<W> + Send>, io::Error)> {
        match GzEncoder::try_finish(&mut self.0) {
            Ok(_) => Ok(GzEncoder::finish(self.0).unwrap()),
            Err(e) => Err((self, e)),
        }
    }
}

/// A wrapper around ZstdEncoder which depends on and implements AsyncWrite.
///
/// The sole purpose of this struct is to work around the orphan rule: you
/// cannot implement a trait in a different crate for a type in a different
/// crate.
pub struct AsyncZstdEncoder<'a, W: AsyncWrite>(ZstdEncoder<'a, W>);

impl<'a, W: AsyncWrite> AsyncZstdEncoder<'a, W> {
    pub fn new(obj: W, level: i32) -> Self {
        // ZstdEncoder::new() should only fail on OOM, so just call unwrap
        // here. The other compression engines effectively do the same thing.
        // TODO: do we want to use the auto_finish variant?
        AsyncZstdEncoder(ZstdEncoder::new(obj, level).unwrap())
    }
}

impl<W> RawEncoder<W> for AsyncZstdEncoder<'static, W>
where
    W: AsyncWrite + Send + 'static,
{
    fn try_finish(
        self: Box<Self>,
    ) -> result::Result<W, (Box<dyn RawEncoder<W> + Send>, io::Error)> {
        match ZstdEncoder::try_finish(self.0) {
            Ok(inner) => Ok(inner),
            Err((encoder, e)) => Err((Box::new(AsyncZstdEncoder(encoder)), e)),
        }
    }
}

impl<'a, W: AsyncWrite> Write for AsyncZstdEncoder<'a, W> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.write(buf)
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}

impl<'a, W: AsyncWrite> AsyncWrite for AsyncZstdEncoder<'a, W> {
    #[inline]
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        self.0.get_mut().shutdown()
    }
}
