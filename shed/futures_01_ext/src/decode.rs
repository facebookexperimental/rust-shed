/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! A layered `Decoder` adapter for `Stream` transformations
//!
//! This module implements an adapter to allow a `tokio_io::codec::Decoder` implementation
//! to transform a `Stream` - specifically, decode from a `Stream` of `Bytes` into some
//! structured type.
//!
//! This allows multiple protocols to be layered and composed with operations on `Streams`,
//! rather than restricting all codec operations to `AsyncRead`/`AsyncWrite` operations on
//! an underlying transport.

use bytes_old::BufMut;
use bytes_old::Bytes;
use bytes_old::BytesMut;
use futures::try_ready;
use futures::Async;
use futures::Poll;
use futures::Stream;
use tokio_io::codec::Decoder;

/// Returns a stream that will yield decoded items that are the result of decoding
/// [Bytes] of the underlying [Stream] by using the provided [Decoder]
pub fn decode<In, Dec>(input: In, decoder: Dec) -> LayeredDecode<In, Dec>
where
    In: Stream<Item = Bytes>,
    Dec: Decoder,
{
    LayeredDecode {
        input,
        decoder,
        // 8KB is a reasonable default
        buf: BytesMut::with_capacity(8 * 1024),
        eof: false,
        is_readable: false,
    }
}

/// Stream returned by the [decode] function
#[derive(Debug)]
pub struct LayeredDecode<In, Dec> {
    input: In,
    decoder: Dec,
    buf: BytesMut,
    eof: bool,
    is_readable: bool,
}

impl<In, Dec> Stream for LayeredDecode<In, Dec>
where
    In: Stream<Item = Bytes>,
    Dec: Decoder,
    Dec::Error: From<In::Error>,
{
    type Item = Dec::Item;
    type Error = Dec::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Dec::Error> {
        // This is adapted from Framed::poll in tokio. This does its own thing
        // because converting the Bytes input stream to an Io object and then
        // running it through Framed is pointless.
        loop {
            if self.is_readable {
                if self.eof {
                    let ret = if self.buf.is_empty() {
                        None
                    } else {
                        self.decoder.decode_eof(&mut self.buf)?
                    };
                    return Ok(Async::Ready(ret));
                }
                if let Some(frame) = self.decoder.decode(&mut self.buf)? {
                    return Ok(Async::Ready(Some(frame)));
                }
                self.is_readable = false;
            }

            assert!(!self.eof);

            match try_ready!(self.input.poll()) {
                Some(v) => {
                    self.buf.reserve(v.len());
                    self.buf.put(v);
                }
                None => self.eof = true,
            }

            self.is_readable = true;
        }
    }
}

impl<In, Dec> LayeredDecode<In, Dec>
where
    In: Stream<Item = Bytes>,
{
    /// Consume this combinator and returned the underlying stream
    #[inline]
    pub fn into_inner(self) -> In {
        // TODO: do we want to check that buf is empty? otherwise we might lose data
        self.input
    }

    /// Returns reference to the underlying stream
    #[inline]
    pub fn get_ref(&self) -> &In {
        &self.input
    }

    /// Returns mutable reference to the underlying stream
    #[inline]
    pub fn get_mut(&mut self) -> &mut In {
        &mut self.input
    }
}

#[cfg(test)]
mod test {
    use std::io;

    use anyhow::Error;
    use anyhow::Result;
    use bytes_old::Bytes;
    use futures::stream;
    use futures::Stream;
    use futures03::compat::Future01CompatExt;

    use super::*;

    #[derive(Default)]
    struct TestDecoder {}

    impl Decoder for TestDecoder {
        type Item = BytesMut;
        type Error = Error;

        fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>> {
            if !buf.is_empty() {
                let expected_len: usize = u8::from_le(buf[0]).into();
                if buf.len() > expected_len {
                    buf.split_to(1);
                    Ok(Some(buf.split_to(expected_len)))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        }
    }

    #[test]
    fn simple() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let decoder = TestDecoder::default();

        let inp = stream::iter_ok::<_, io::Error>(vec![Bytes::from(&b"\x0Dhello, world!"[..])]);

        let dec = decode(inp, decoder);
        let out = Vec::new();

        let xfer = dec
            .map_err::<(), _>(|err| {
                panic!("bad = {err}");
            })
            .forward(out);

        let (_, out) = runtime.block_on(xfer.compat()).unwrap();
        let out = out
            .into_iter()
            .flat_map(|x| x.as_ref().to_vec())
            .collect::<Vec<_>>();
        assert_eq!(out, b"hello, world!");
    }

    #[test]
    fn large() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let decoder = TestDecoder::default();

        let inp =
            stream::iter_ok::<_, io::Error>(vec![Bytes::from("\x0Dhello, world!".repeat(5000))]);

        let dec = decode(inp, decoder);
        let out = Vec::new();

        let xfer = dec
            .map_err::<(), _>(|err| {
                panic!("bad = {err}");
            })
            .forward(out);

        let (_, out) = runtime.block_on(xfer.compat()).unwrap();
        let out = out
            .into_iter()
            .flat_map(|x| x.as_ref().to_vec())
            .collect::<Vec<_>>();

        assert_eq!(out, "hello, world!".repeat(5000).as_bytes());
    }

    #[test]
    fn partial() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let decoder = TestDecoder::default();

        let inp = stream::iter_ok::<_, io::Error>(vec![
            Bytes::from(&b"\x0Dhel"[..]),
            Bytes::from(&b"lo, world!"[..]),
        ]);

        let dec = decode(inp, decoder);
        let out = Vec::new();

        let xfer = dec
            .map_err::<(), _>(|err| {
                panic!("bad = {err}");
            })
            .forward(out);

        let (_, out) = runtime.block_on(xfer.compat()).unwrap();
        let out = out
            .into_iter()
            .flat_map(|x| x.as_ref().to_vec())
            .collect::<Vec<_>>();
        assert_eq!(out, b"hello, world!");
    }
}
