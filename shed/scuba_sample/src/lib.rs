/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]
#![allow(elided_lifetimes_in_paths)]

//! Defines the [sample::ScubaSample] structure.
//!
//! Scuba is a system that can aggregate log lines in a structured manner, this
//! crates also defines means to serialize the dataset into json format
//! understandable by Scuba.

pub mod sample;
pub mod value;

pub use scuba_sample_derive::*;

pub use crate::sample::Error;
pub use crate::sample::ScubaSample;
pub use crate::sample::StructuredSample;
pub use crate::sample::TryFromSample;
pub use crate::value::ScubaValue;
