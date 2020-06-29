/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::{Error, Result};
use bytes::BytesMut;
use fbthrift::{Framing, FramingDecoded, FramingEncodedFinal, Transport};
use fbthrift_framed::FramedTransport;
use futures::compat::Future01CompatExt;
use futures::future::{FutureExt, TryFutureExt};
use std::future::Future;
use std::io::{self, Cursor};
use std::marker::Sync;
use std::net::SocketAddr;
use std::pin::Pin;
use tokio_core::net::TcpStream;
use tokio_core::reactor::Handle;
use tokio_proto::pipeline::ClientService;
use tokio_proto::TcpClient;
use tokio_service::Service;

/// ```ignore
/// let transport = TcpTransport::connect(addr, handle).await?;
/// let client = fb303::client::FacebookService::new(CompactProtocol, transport);
/// ```
pub struct TcpTransport {
    service: ClientService<TcpStream, FramedTransport>,
}

impl TcpTransport {
    pub fn connect(
        addr: &SocketAddr,
        handle: &Handle,
    ) -> impl Future<Output = Result<Self, io::Error>> {
        TcpClient::new(FramedTransport)
            .connect(addr, handle)
            .compat()
            .map_ok(|service| TcpTransport { service })
    }
}

impl Framing for TcpTransport {
    type EncBuf = BytesMut;
    type DecBuf = Cursor<bytes_old::Bytes>;
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
        self.service.call(req).compat().map_err(Error::new).boxed()
    }
}

unsafe impl Sync for TcpTransport {}
