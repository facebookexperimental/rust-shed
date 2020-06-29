/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use crate::FramedTransport;
use bytes::Bytes;
use futures::{stream, Stream};
use std::io::{self, Cursor};
use tokio::runtime::Runtime;
use tokio_proto::pipeline::ClientProto;

#[test]
fn framed_transport_encode() {
    let buf = Cursor::new(Vec::with_capacity(32));

    let trans = FramedTransport.bind_transport(buf).unwrap();

    let input = Bytes::from(vec![0u8, 1, 2, 3, 4, 5, 6, 7]);

    let stream = stream::once::<_, io::Error>(Ok(input));

    let fut = stream.forward(trans);

    let mut runtime = Runtime::new().unwrap();

    let (_stream, trans) = runtime.block_on(fut).unwrap();

    let expected = vec![0, 0, 0, 8, 0, 1, 2, 3, 4, 5, 6, 7];

    let encoded = trans.into_inner().into_inner();

    assert_eq!(encoded, expected, "encoded frame not equal");
}

#[test]
fn framed_transport_decode() {
    let buf = Cursor::new(vec![0u8, 0, 0, 8, 0, 1, 2, 3, 4, 5, 6, 7]);

    let trans = FramedTransport.bind_transport(buf).unwrap();

    let fut = trans.collect();

    let mut runtime = Runtime::new().unwrap();

    let mut decoded = runtime.block_on(fut).unwrap();

    let decoded = decoded.pop().unwrap();

    let expected = vec![0u8, 1, 2, 3, 4, 5, 6, 7];

    assert_eq!(decoded.into_inner(), expected, "decoded frame not equal");
}

#[test]
fn framed_transport_decode_incomplete_frame() {
    // Promise 8, deliver 7
    let buf = Cursor::new(vec![0u8, 0, 0, 8, 0, 1, 2, 3, 4, 5, 6]);

    let trans = FramedTransport.bind_transport(buf).unwrap();

    let fut = trans.collect();

    let mut runtime = Runtime::new().unwrap();

    assert!(
        runtime.block_on(fut).is_err(),
        "returned Ok with bytes left on stream"
    );
}

#[test]
fn framed_transport_decode_incomplete_header() {
    // Promise 8, deliver 7
    let buf = Cursor::new(vec![0u8, 0, 0]);

    let trans = FramedTransport.bind_transport(buf).unwrap();

    let fut = trans.collect();

    let mut runtime = Runtime::new().unwrap();

    assert!(
        runtime.block_on(fut).is_err(),
        "returned Ok with bytes left on stream"
    );
}
