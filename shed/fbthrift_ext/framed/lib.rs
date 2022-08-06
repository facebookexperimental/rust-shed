/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#[cfg(test)]
mod tests;

use std::io;
use std::io::Cursor;

use byteorder::BigEndian;
use byteorder::ByteOrder;
use bytes::BufMut as _;
use bytes::Bytes;
use tokio_util::codec::Decoder;
use tokio_util::codec::Encoder;

pub struct FramedTransport;

impl Encoder<Bytes> for FramedTransport {
    type Error = io::Error;

    fn encode(&mut self, item: Bytes, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        dst.reserve(4 + item.len());
        dst.put_u32(item.len() as u32);
        dst.put(item.as_ref());
        Ok(())
    }
}

impl Decoder for FramedTransport {
    type Item = Cursor<Bytes>;
    type Error = io::Error;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Wait for at least a frame header
        if src.len() < 4 {
            return Ok(None);
        }

        // Peek at first 4 bytes, don't advance src buffer
        let len = BigEndian::read_u32(&src[..4]) as usize;

        // Make sure we have all the bytes we were promised
        if src.len() < 4 + len {
            return Ok(None);
        }

        // Drain 4 bytes from src
        let _ = src.split_to(4).as_ref();

        // Take len bytes, advancing src
        let res = src.split_to(len);

        // Convert this to the Bytes 1.x implementation we are using.
        let res = Bytes::copy_from_slice(res.as_ref());

        Ok(Some(Cursor::new(res)))
    }
}
