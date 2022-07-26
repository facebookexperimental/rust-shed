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

//! Defines the [sample::ScubaSample] structure and the
//! [builder::ScubaSampleBuilder] helper structure to build a sample for
//! Scuba.
//!
//! Scuba is a system that can aggregate log lines in a structured manner, this
//! crates also defines means to serialize the dataset into json format
//! understandable by Scuba.

pub mod builder;
pub mod client;
pub mod sample;
pub mod value;

mod sampling;

pub use crate::builder::ScubaSampleBuilder;
pub use crate::client::ScubaClient;
pub use crate::sample::ScubaSample;
pub use crate::sample::StructuredSample;
pub use crate::sampling::Sampling;
pub use crate::value::ScubaValue;
pub use scuba_sample_derive::*;
