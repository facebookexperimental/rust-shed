/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]
#![allow(elided_lifetimes_in_paths)]

//! Defines [client::ScubaClient] helper structure.

pub mod client;

use scuba_sample::*;

pub use crate::client::ScubaClient;
