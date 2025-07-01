/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
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
