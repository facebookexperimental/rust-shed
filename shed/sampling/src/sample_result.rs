/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

/// Indicates whether a sample should be logged.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SampleResult {
    /// The sample should be sent to wherever due to its sampling result.
    Include,
    /// The sample should not be sent to wherever due to its sampling result.
    Exclude,
}
