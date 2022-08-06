/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! This crate adds [Compressor] and [Decompressor] wrappers around some common
//! compression libraries. Those wrappers implement [tokio_io::AsyncWrite] and
//! [tokio_io::AsyncRead] respectively so they might be used efficiently in an
//! asynchronous contexts.

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]

mod compressor;
mod decompressor;
pub mod membuf;
pub mod metered;
mod raw;
mod retry;

#[cfg(test)]
mod test;

pub use bzip2::Compression as Bzip2Compression;
pub use flate2::Compression as FlateCompression;

pub use crate::compressor::Compressor;
pub use crate::compressor::CompressorType;
pub use crate::decompressor::Decompressor;
pub use crate::decompressor::DecompressorType;
