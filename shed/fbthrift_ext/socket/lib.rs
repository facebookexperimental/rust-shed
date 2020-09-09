/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![cfg(not(target_os = "windows"))]

use anyhow::{Error, Result};
use bytes::{Bytes, BytesMut};
use fbthrift::{Framing, FramingDecoded, FramingEncodedFinal, Transport};
use fbthrift_framed::FramedTransport;
use futures::compat::Future01CompatExt;
use futures::future::{FutureExt, TryFutureExt};
use std::future::Future;
use std::io::Cursor;
use std::pin::Pin;
use std::sync::Arc;
use tokio::net::UnixStream;
use tokio::sync::Mutex;
use tokio_core::reactor::Handle;
use tokio_proto::pipeline::ClientService;
use tokio_proto::BindClient;
use tokio_service::Service as _;
use tokio_tower::pipeline::client::Client;
use tokio_util::codec::{Decoder, Framed};
use tower_service::Service;

pub mod util;

use crate::util::poll_with_lock;

/// ```ignore
/// let stream = tokio_uds::UnixStream::connect(path, handle)?;
/// let transport = SocketTransportLegacy::new(handle, stream);
/// let client = fb303::client::FacebookService::new(CompactProtocol, transport);
/// ```
pub struct SocketTransportLegacy {
    service: ClientService<tokio_uds::UnixStream, FramedTransport>,
}

impl SocketTransportLegacy {
    pub fn new(handle: &Handle, stream: tokio_uds::UnixStream) -> Self {
        SocketTransportLegacy {
            service: FramedTransport.bind_client(&handle, stream),
        }
    }
}

impl Framing for SocketTransportLegacy {
    type EncBuf = BytesMut;
    type DecBuf = Cursor<bytes_old::Bytes>;
    type Meta = ();

    fn enc_with_capacity(cap: usize) -> Self::EncBuf {
        BytesMut::with_capacity(cap)
    }

    fn get_meta(&self) {}
}

impl Transport for SocketTransportLegacy {
    fn call(
        &self,
        req: FramingEncodedFinal<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<FramingDecoded<Self>>> + Send + 'static>> {
        self.service.call(req).compat().map_err(Error::new).boxed()
    }
}

/// ```ignore
/// let stream = tokio_uds::UnixStream::connect(path)?;
/// let transport = SocketTransport::new(stream);
/// let client = fb303::client::FacebookService::new(CompactProtocol, transport);
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
