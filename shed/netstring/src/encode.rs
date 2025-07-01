/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::fmt::Write;
use std::marker::PhantomData;

use anyhow::Error;
use anyhow::Result;
use bytes::BufMut;
use bytes::BytesMut;
use tokio_util::codec::Encoder;

/// A Netstring encoder.
///
/// The items can be anything that can be referenced as a `[u8]`.
#[derive(Debug)]
pub struct NetstringEncoder<Out>
where
    Out: AsRef<[u8]>,
{
    _marker: PhantomData<Out>,
}

impl<Out> Default for NetstringEncoder<Out>
where
    Out: AsRef<[u8]>,
{
    fn default() -> Self {
        NetstringEncoder {
            _marker: PhantomData,
        }
    }
}

impl<Out> Encoder<Out> for NetstringEncoder<Out>
where
    Out: AsRef<[u8]>,
{
    type Error = Error;

    fn encode(&mut self, msg: Out, buf: &mut BytesMut) -> Result<()> {
        let msg = msg.as_ref();

        // Assume that 20 digits is long enough for the length
        // <len> ':' <payload> ','
        buf.reserve(20 + 1 + msg.len() + 1);
        write!(buf, "{}:", msg.len()).expect("write to slice failed?");
        buf.put_slice(msg);
        buf.put_u8(b',');
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use bytes::BytesMut;
    use quickcheck::quickcheck;
    use tokio_util::codec::Decoder;

    use super::*;
    use crate::NetstringDecoder;

    #[test]
    fn encode_simple() {
        let mut buf = BytesMut::with_capacity(1);

        let mut codec = NetstringEncoder::<&[u8]>::default();

        assert!(codec.encode(b"hello, world", &mut buf).is_ok());
        assert_eq!(buf.as_ref(), b"12:hello, world,");
    }

    #[test]
    fn encode_zero() {
        let mut buf = BytesMut::with_capacity(1);

        let mut codec = NetstringEncoder::<&[u8]>::default();

        assert!(codec.encode(b"", &mut buf).is_ok());
        assert_eq!(buf.as_ref(), b"0:,");
    }

    #[test]
    fn encode_multiple() {
        let mut buf = BytesMut::with_capacity(1);

        let mut codec = NetstringEncoder::<&[u8]>::default();

        assert!(codec.encode(b"hello, ", &mut buf).is_ok());
        assert!(codec.encode(b"world!", &mut buf).is_ok());
        assert_eq!(buf.as_ref(), b"7:hello, ,6:world!,");
    }

    quickcheck! {
        fn roundtrip(s: Vec<u8>) -> bool {
            let mut buf = BytesMut::with_capacity(1);
            let mut enc = NetstringEncoder::default();

            assert!(enc.encode(&s, &mut buf).is_ok(), "encode failed");

            let mut dec = NetstringDecoder::default();
            let out = dec.decode(&mut buf).expect("decode failed").expect("incomplete");

            s == out
        }
    }
}
