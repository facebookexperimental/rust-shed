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
use bytes::BytesMut;
use fbthrift::{Framing, FramingDecoded, FramingEncodedFinal, Transport};
use fbthrift_framed::FramedTransport;
use futures::compat::Future01CompatExt;
use futures::future::{FutureExt, TryFutureExt};
use std::future::Future;
use std::io::Cursor;
use std::pin::Pin;
use tokio_core::reactor::Handle;
use tokio_proto::pipeline::ClientService;
use tokio_proto::BindClient;
use tokio_service::Service;
use tokio_uds::UnixStream;

pub mod util;

/// ```ignore
/// let stream = tokio_uds::UnixStream::connect(path, handle)?;
/// let transport = SocketTransport::new(handle, stream);
/// let client = fb303::client::FacebookService::new(CompactProtocol, transport);
/// ```
pub struct SocketTransport {
    service: ClientService<UnixStream, FramedTransport>,
}

impl SocketTransport {
    pub fn new(handle: &Handle, stream: UnixStream) -> Self {
        SocketTransport {
            service: FramedTransport.bind_client(&handle, stream),
        }
    }
}

impl Framing for SocketTransport {
    type EncBuf = BytesMut;
    type DecBuf = Cursor<bytes_old::Bytes>;
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
        self.service.call(req).compat().map_err(Error::new).boxed()
    }
}
