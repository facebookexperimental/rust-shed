/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::io::Cursor;

use bytes::Bytes;
use futures::stream;
use futures::stream::StreamExt;
use futures::stream::TryStreamExt;
use tokio_util::codec::Decoder;

use crate::FramedTransport;

#[tokio::test]
async fn framed_transport_encode() {
    let buf = Cursor::new(Vec::with_capacity(32));
    let mut trans = FramedTransport.framed(buf);

    let input = Bytes::from(vec![0u8, 1, 2, 3, 4, 5, 6, 7]);
    let stream = stream::once(async { Ok(input) });

    stream.forward(&mut trans).await.unwrap();

    let expected = vec![0, 0, 0, 8, 0, 1, 2, 3, 4, 5, 6, 7];
    let encoded = trans.into_inner().into_inner();
    assert_eq!(encoded, expected, "encoded frame not equal");
}

#[tokio::test]
async fn framed_transport_decode() {
    let buf = Cursor::new(vec![0u8, 0, 0, 8, 0, 1, 2, 3, 4, 5, 6, 7]);
    let trans = FramedTransport.framed(buf);

    let mut decoded = trans.collect::<Vec<_>>().await;
    let decoded = decoded.pop().unwrap().unwrap();

    let expected = vec![0u8, 1, 2, 3, 4, 5, 6, 7];
    assert_eq!(decoded.into_inner(), expected, "decoded frame not equal");
}

#[tokio::test]
async fn framed_transport_decode_incomplete_frame() {
    // Promise 8, deliver 7
    let buf = Cursor::new(vec![0u8, 0, 0, 8, 0, 1, 2, 3, 4, 5, 6]);
    let transport = FramedTransport.framed(buf);
    assert!(
        transport.try_collect::<Vec<_>>().await.is_err(),
        "returned Ok with bytes left on stream"
    );
}

#[tokio::test]
async fn framed_transport_decode_incomplete_header() {
    // Promise 8, deliver 7
    let buf = Cursor::new(vec![0u8, 0, 0]);
    let transport = FramedTransport.framed(buf);
    assert!(
        transport.try_collect::<Vec<_>>().await.is_err(),
        "returned Ok with bytes left on stream"
    );
}
