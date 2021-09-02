/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::{Error, Result};
use bytes::{Bytes, BytesMut};
use fbthrift::{Framing, FramingDecoded, FramingEncodedFinal, Transport};
use fbthrift_framed::FramedTransport;
use fbthrift_util::poll_with_lock;
use futures::future::FutureExt;
use std::future::Future;
use std::io::Cursor;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tower::pipeline::client::Client;
use tokio_uds_compat::UnixStream;
use tokio_util::codec::{Decoder, Framed};
use tower_service::Service;

/// ```ignore
/// let stream = tokio::net::UnixStream::connect(path)?;
/// let transport = SocketTransport::new(stream);
/// let client = <dyn fb303::client::FacebookService>::new(CompactProtocol, transport);
/// ```
pub struct SocketTransport {
    service: Arc<Mutex<Client<Framed<UnixStream, FramedTransport>, Error, Bytes>>>,
}

impl SocketTransport {
    pub fn new(stream: UnixStream) -> Self {
        SocketTransport {
            service: Arc::new(Mutex::new(Client::new(FramedTransport.framed(stream)))),
        }
    }
}

impl Framing for SocketTransport {
    type EncBuf = BytesMut;
    type DecBuf = Cursor<Bytes>;
    type Meta = ();

    fn enc_with_capacity(cap: usize) -> Self::EncBuf {
        BytesMut::with_capacity(cap)
    }

    fn get_meta(&self) {}
}

impl Transport for SocketTransport {
    fn call(
        &self,
        _service_name: const_cstr::ConstCStr,
        _fn_name: const_cstr::ConstCStr,
        req: FramingEncodedFinal<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<FramingDecoded<Self>>> + Send + 'static>> {
        let svc = self.service.clone();
        (async move {
            let locked = poll_with_lock(&svc, |locked, ctx| locked.poll_ready(ctx)).await;
            match locked {
                Ok(mut locked) => locked.call(req).await,
                Err(e) => Err(e),
            }
        })
        .boxed()
    }
}
