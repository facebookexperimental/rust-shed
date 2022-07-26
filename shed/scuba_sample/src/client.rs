/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! See the [ScubaSample] documentation

use crate::ScubaSample;

/// A Scuba Client.
pub struct ScubaClient {}

impl ScubaClient {
    /// Create a Scuba client instance.
    pub fn new(_fb: fbinit::FacebookInit, _dataset: &str) -> Self {
        ScubaClient {}
    }

    /// Log to Scuba.
    pub fn log(&self, _sample: &ScubaSample) {}
}
