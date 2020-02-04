/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, intra_doc_link_resolution_failure)]

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
pub use scuba::*;

#[cfg(not(fbcode_build))]
pub use scuba_sample::*;
