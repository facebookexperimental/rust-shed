/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#[cfg(unix)]
pub use tokio::net::UnixStream;

#[cfg(windows)]
mod windows {
    use std::io;
    use std::path::Path;
    use std::pin::Pin;

    /// Compat layer for providing UNIX domain socket on Windows
    use async_io::Async;
    use tokio::io::AsyncRead;
    use tokio::io::AsyncWrite;
    use tokio::io::ReadBuf;

    pub struct UnixStream(Async<uds_windows::UnixStream>);

    impl UnixStream {
        pub async fn connect<P: AsRef<Path>>(path: P) -> io::Result<Self> {
            let stream = uds_windows::UnixStream::connect(path)?;
            let stream = Async::new(stream)?;

            Ok(UnixStream(stream))
        }

        fn inner_mut(self: Pin<&mut Self>) -> Pin<&mut Async<uds_windows::UnixStream>> {
            Pin::new(&mut self.get_mut().0)
        }
    }

    impl AsyncRead for UnixStream {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> std::task::Poll<Result<(), io::Error>> {
            let result =
                futures::AsyncRead::poll_read(self.inner_mut(), cx, buf.initialize_unfilled());

            match result {
                std::task::Poll::Ready(Ok(written)) => {
                    tracing::trace!(?written, "UnixStream::poll_read");
                    buf.set_filled(written);
                    std::task::Poll::Ready(Ok(()))
                }
                std::task::Poll::Ready(Err(e)) => std::task::Poll::Ready(Err(e)),
                std::task::Poll::Pending => std::task::Poll::Pending,
            }
        }
    }

    impl AsyncWrite for UnixStream {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<Result<usize, io::Error>> {
            futures::AsyncWrite::poll_write(self.inner_mut(), cx, buf)
        }

        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), io::Error>> {
            futures::AsyncWrite::poll_flush(self.inner_mut(), cx)
        }

        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), io::Error>> {
            futures::AsyncWrite::poll_close(self.inner_mut(), cx)
        }
    }
}

#[cfg(windows)]
pub use self::windows::UnixStream;
