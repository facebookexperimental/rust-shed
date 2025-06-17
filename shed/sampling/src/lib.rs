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

//! Contains logic for sampling items. For example for use in Scuba, see
//! [`scuba_sample`].

mod sample_result;
mod sampleable;
mod sampling;

pub use crate::sample_result::SampleResult;
pub use crate::sampleable::Sampleable;
pub use crate::sampling::Sampling;
