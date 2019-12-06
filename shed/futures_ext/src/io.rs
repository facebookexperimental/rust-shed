/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

//! Extends the functionality of [std::io] and [::tokio_io]

use std::io::{self, Read, Write};

use futures::Poll;
use tokio_io::{AsyncRead, AsyncWrite};

/// Like [::futures::future::Either], combines two different types implementing
/// the same trait into a single type.
///
/// The traits supported here are:
/// - [Read]
/// - [Write]
/// - [AsyncRead]
/// - [AsyncWrite]
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Either<A, B> {
    /// First branch of the type
    A(A),
    /// Second branch of the type
    B(B),
}

impl<A: Read, B: Read> Read for Either<A, B> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Either::A(ref mut inner) => inner.read(buf),
            Either::B(ref mut inner) => inner.read(buf),
        }
    }
}

impl<A: AsyncRead, B: AsyncRead> AsyncRead for Either<A, B> {
    unsafe fn prepare_uninitialized_buffer(&self, buf: &mut [u8]) -> bool {
        match self {
            Either::A(ref inner) => inner.prepare_uninitialized_buffer(buf),
            Either::B(ref inner) => inner.prepare_uninitialized_buffer(buf),
        }
    }
}

impl<A: Write, B: Write> Write for Either<A, B> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Either::A(ref mut inner) => inner.write(buf),
            Either::B(ref mut inner) => inner.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Either::A(ref mut inner) => inner.flush(),
            Either::B(ref mut inner) => inner.flush(),
        }
    }
}

impl<A: AsyncWrite, B: AsyncWrite> AsyncWrite for Either<A, B> {
    fn shutdown(&mut self) -> Poll<(), io::Error> {
        match self {
            Either::A(ref mut inner) => inner.shutdown(),
            Either::B(ref mut inner) => inner.shutdown(),
        }
    }
}
