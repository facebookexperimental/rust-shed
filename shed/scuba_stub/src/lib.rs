/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]

//! Defines the [sample::ScubaSample] structure and the
//! [builder::ScubaSampleBuilder] helper structure to build a sample for
//! Scuba.
//!
//! Scuba is a system that can aggregate log lines in a structured manner, this
//! crates also defines means to serialize the dataset into json format
//! understandable by Scuba.
//!
//! Facebook only: This crate also defines the default Scuba client to be used
//! to send data to Scuba and some helper methods to interact with the client.
//! For non-fbcode builds there is no client yet.

#[cfg(fbcode_build)]
pub use fb_scuba::*;
#[cfg(not(fbcode_build))]
pub use scuba_sample::*;
