/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::ffi::CStr;
use std::future::Future;
use std::io::Cursor;
use std::pin::Pin;
use std::sync::Arc;

use anyhow::Error;
use anyhow::Result;
use bytes::Bytes;
use bytes::BytesMut;
use fbthrift::Framing;
use fbthrift::FramingDecoded;
use fbthrift::FramingEncodedFinal;
use fbthrift::Transport;
use fbthrift_framed::FramedTransport;
use fbthrift_util::poll_with_lock;
use futures::future::FutureExt;
use tokio::io::AsyncRead;
use tokio::io::AsyncWrite;
use tokio::sync::Mutex;
use tokio_tower::pipeline::client::Client;
use tokio_util::codec::Decoder;
use tokio_util::codec::Framed;
use tower_service::Service;

/// ```ignore
/// let stream = tokio::net::UnixStream::connect(path)?;
/// let transport = SocketTransport::new(stream);
/// let client = <dyn fb303::client::FacebookService>::new(CompactProtocol, transport);
/// ```
pub struct SocketTransport<T>
where
    T: AsyncRead + AsyncWrite + Sized + Send + 'static,
{
    service: Arc<Mutex<Client<Framed<T, FramedTransport>, Error, Bytes>>>,
}

impl<T> SocketTransport<T>
where
    T: AsyncRead + AsyncWrite + Sized + Send + 'static,
{
    pub fn new(stream: T) -> Self {
        SocketTransport {
            service: Arc::new(Mutex::new(Client::new(FramedTransport.framed(stream)))),
        }
    }
}

impl<T> Framing for SocketTransport<T>
where
    T: AsyncRead + AsyncWrite + Sized + Send + 'static,
{
    type EncBuf = BytesMut;
    type DecBuf = Cursor<Bytes>;

    fn enc_with_capacity(cap: usize) -> Self::EncBuf {
        BytesMut::with_capacity(cap)
    }
}

impl<T> Transport for SocketTransport<T>
where
    T: AsyncRead + AsyncWrite + Sized + Send + 'static,
{
    type RpcOptions = ();

    fn call(
        &self,
        _service_name: &'static CStr,
        _fn_name: &'static CStr,
        req: FramingEncodedFinal<Self>,
        _rpc_options: Self::RpcOptions,
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
