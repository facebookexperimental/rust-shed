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
use futures::compat::Future01CompatExt;
use futures::future::{FutureExt, TryFutureExt};
use std::future::Future;
use std::io::{self, Cursor};
use std::marker::Sync;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio_core::net::TcpStream as TcpStreamLegacy;
use tokio_core::reactor::Handle;
use tokio_proto::pipeline::ClientService;
use tokio_proto::TcpClient;
use tokio_service::Service as _;
use tokio_tower::pipeline::client::Client;
use tokio_util::codec::{Decoder, Framed};
use tower_service::Service;

/// ```ignore
/// let transport = TcpTransportLegacy::connect(addr, handle).await?;
/// let client = fb303::client::FacebookService::new(CompactProtocol, transport);
/// ```
pub struct TcpTransportLegacy {
    service: ClientService<TcpStreamLegacy, FramedTransport>,
}

impl TcpTransportLegacy {
    pub fn connect(
        addr: &SocketAddr,
        handle: &Handle,
    ) -> impl Future<Output = Result<Self, io::Error>> {
        TcpClient::new(FramedTransport)
            .connect(addr, handle)
            .compat()
            .map_ok(|service| TcpTransportLegacy { service })
    }
}

impl Framing for TcpTransportLegacy {
    type EncBuf = BytesMut;
    type DecBuf = Cursor<bytes_old::Bytes>;
    type Meta = ();

    fn enc_with_capacity(cap: usize) -> Self::EncBuf {
        BytesMut::with_capacity(cap)
    }

    fn get_meta(&self) {}
}

impl Transport for TcpTransportLegacy {
    fn call(
        &self,
        req: FramingEncodedFinal<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<FramingDecoded<Self>>> + Send + 'static>> {
        self.service.call(req).compat().map_err(Error::new).boxed()
    }
}

unsafe impl Sync for TcpTransportLegacy {}

/// ```ignore
/// let stream = tokio::net::TcpStream::connect(path)?;
/// let transport = TcpTransport::new(stream);
/// let client = fb303::client::FacebookService::new(CompactProtocol, transport);
/// ```
pub struct TcpTransport {
    service: Arc<Mutex<Client<Framed<TcpStream, FramedTransport>, Error, Bytes>>>,
}

impl TcpTransport {
    pub fn new(stream: TcpStream) -> Self {
        TcpTransport {
            service: Arc::new(Mutex::new(Client::new(FramedTransport.framed(stream)))),
        }
    }
}

impl Framing for TcpTransport {
    type EncBuf = BytesMut;
    type DecBuf = Cursor<Bytes>;
    type Meta = ();

    fn enc_with_capacity(cap: usize) -> Self::EncBuf {
        BytesMut::with_capacity(cap)
    }

    fn get_meta(&self) {}
}

impl Transport for TcpTransport {
    fn call(
        &self,
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
