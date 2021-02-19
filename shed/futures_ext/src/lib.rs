/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, broken_intra_doc_links)]

//! Crate extending functionality of [`futures`] crate

pub mod future;
pub mod stream;

pub use crate::future::{FbFutureExt, FbTryFutureExt};
pub use crate::stream::{BufferedParams, FbStreamExt, FbTryStreamExt};
