/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#[cfg(test)]
mod tests;

use byteorder::{BigEndian, ByteOrder};
use bytes::Bytes;
use bytes_ext::BytesCompat;
use bytes_old::BufMut as _;
use std::io::{self, Cursor};
use tokio_codec::{Decoder, Encoder, Framed};
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_proto::pipeline::{ClientProto, ServerProto};

pub struct FramedTransport;
impl<T> ClientProto<T> for FramedTransport
where
    T: AsyncRead + AsyncWrite + 'static,
{
    type Request = Bytes;
    type Response = Cursor<bytes_old::Bytes>;
    type Transport = Framed<T, FramedTransportCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(FramedTransportCodec.framed(io))
    }
}

impl<T> ServerProto<T> for FramedTransport
where
    T: AsyncRead + AsyncWrite + 'static,
{
    type Request = Cursor<bytes_old::Bytes>;
    type Response = Bytes;
    type Transport = Framed<T, FramedTransportCodec>;
    type BindTransport = Result<Self::Transport, io::Error>;

    fn bind_transport(&self, io: T) -> Self::BindTransport {
        Ok(FramedTransportCodec.framed(io))
    }
}

pub struct FramedTransportCodec;
impl Encoder for FramedTransportCodec {
    type Item = Bytes;
    type Error = io::Error;

    fn encode(
        &mut self,
        item: Self::Item,
        dst: &mut bytes_old::BytesMut,
    ) -> Result<(), Self::Error> {
        dst.reserve(4 + item.len());
        dst.put_u32_be(item.len() as u32);
        dst.put(BytesCompat::new(item));
        Ok(())
    }
}

impl Decoder for FramedTransportCodec {
    type Item = Cursor<bytes_old::Bytes>;
    type Error = io::Error;

    fn decode(&mut self, src: &mut bytes_old::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
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

        Ok(Some(Cursor::new(res.freeze())))
    }
}
