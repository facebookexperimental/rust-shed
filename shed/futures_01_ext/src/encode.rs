/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! A layered `Encoder` adapter for `Stream` transformations
//!
//! This module implements an adapter to allow a `tokio_io::codec::Encoder` implementation
//! to transform a `Stream` - specifically, encode from some structured type to a `Stream`
//! of `Bytes`.
//!
//! This allows multiple protocols to be layered and composed with operations on `Streams`,
//! rather than restricting all codec operations to `AsyncRead`/`AsyncWrite` operations on
//! an underlying transport.

use bytes_old::Bytes;
use bytes_old::BytesMut;
use futures::Async;
use futures::Poll;
use futures::Stream;
use tokio_io::codec::Encoder;

const INITIAL_CAPACITY: usize = 8192;
const HEADROOM: usize = 512;
const HIGHWATER: usize = INITIAL_CAPACITY - HEADROOM;

/// Returns a stream that will yield [Bytes] that are the result of encoding
/// items of the underlying [Stream] by using the provided [Encoder]
pub fn encode<In, Enc>(inp: In, enc: Enc) -> LayeredEncoder<In, Enc>
where
    In: Stream,
    Enc: Encoder<Item = In::Item>,
{
    LayeredEncoder {
        inp,
        enc,
        eof: false,
        buf: BytesMut::with_capacity(INITIAL_CAPACITY),
    }
}

/// Stream returned by the [encode] function
pub struct LayeredEncoder<In, Enc> {
    inp: In,       // source
    enc: Enc,      // encoder
    eof: bool,     // source finished
    buf: BytesMut, // accumulated output
}

// Encode Items into Bytes blobs
impl<In, Enc> Stream for LayeredEncoder<In, Enc>
where
    In: Stream,
    Enc: Encoder<Item = In::Item>,
    Enc::Error: From<In::Error>,
{
    type Item = Bytes;
    type Error = Enc::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // Loop filling the buffer until either the input is done or
        // the output buffer is large enough.
        loop {
            let mut push = false;

            if !self.eof {
                match self.inp.poll()? {
                    Async::Ready(None) => self.eof = true,
                    Async::Ready(Some(item)) => self.enc.encode(item, &mut self.buf)?,
                    Async::NotReady => push = true, // no input -> push output to avoid deadlock
                }
            }

            let len = self.buf.len();
            let push = push || len > HIGHWATER || self.eof;

            if push {
                match (len == 0, self.eof) {
                    // Input finished, no output
                    (true, true) => return Ok(Async::Ready(None)),
                    // Input not finished, no output
                    (true, false) => return Ok(Async::NotReady),
                    // Something to output
                    (false, eof) => {
                        let ret = self.buf.split_to(len);
                        // regrow buffer if we're not done
                        if !eof {
                            self.buf.reserve(HIGHWATER)
                        }
                        return Ok(Async::Ready(Some(ret.freeze())));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bytes_old::BigEndian;
    use bytes_old::BufMut;
    use bytes_old::ByteOrder;
    use futures::Future;
    use quickcheck::quickcheck;
    use std::io;
    use std::vec;

    struct EncU16;

    impl Encoder for EncU16 {
        type Item = u16;
        type Error = io::Error;

        fn encode(&mut self, item: Self::Item, dst: &mut BytesMut) -> Result<(), Self::Error> {
            dst.reserve(2);
            dst.put_u16_be(item);

            Ok(())
        }
    }

    struct TestStream<T> {
        iter: vec::IntoIter<Option<T>>,
    }

    impl<T> TestStream<T> {
        fn new(v: Vec<Option<T>>) -> Self {
            TestStream {
                iter: v.into_iter(),
            }
        }
    }

    impl<T> Stream for TestStream<T> {
        type Item = T;
        type Error = io::Error;

        fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
            match self.iter.next() {
                None => Ok(Async::Ready(None)),
                Some(None) => Ok(Async::NotReady),
                Some(Some(v)) => Ok(Async::Ready(Some(v))),
            }
        }
    }

    #[test]
    fn simple() {
        let s = TestStream::new(vec![Some(0u16), Some(4), Some(5), Some(6), Some(256)]);
        let enc = encode::<_, _>(s, EncU16);
        let res = enc.collect().wait().expect("collect failed");
        let res: Vec<u8> = res.into_iter().flatten().collect();
        assert_eq!(res.as_slice(), &[0, 0, 0, 4, 0, 5, 0, 6, 1, 0][..]);
    }

    quickcheck! {
        fn encoding(v: Vec<Option<u16>>) -> bool {
            let s = TestStream::new(v.clone());
            let mut enc = encode::<_, _>(s, EncU16);

            let mut res = Vec::new();
            loop {
                match enc.poll().expect("poll failed") {
                    Async::NotReady => (), // "spin"
                    Async::Ready(None) => break,
                    Async::Ready(Some(v)) => res.push(v),
                }
            }

            let res: Vec<u8> = res.into_iter().flatten().collect();
            let input: Vec<u16> = v.into_iter().flatten().collect();
            assert_eq!(input.len() * 2, res.len());

            let mut output = Vec::new();
            for i in 0..input.len() {
                output.push(<BigEndian as ByteOrder>::read_u16(&res[i * 2..i * 2 + 2]))
            }

            input == output
        }
    }
}
