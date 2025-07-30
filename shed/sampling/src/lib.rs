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

//! Contains logic for sampling items. For example for use in Scuba, see
//! the `scuba_sample` crate.

mod sample_result;
mod sampleable;
mod sampling;

pub use crate::sample_result::SampleResult;
pub use crate::sampleable::Sampleable;
pub use crate::sampling::Sampling;
