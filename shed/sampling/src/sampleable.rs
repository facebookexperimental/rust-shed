/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::num::NonZeroU64;

/// Indicate that a type can have a sample rate associated with it.
///
/// For example, samples that are uploaded to a time-series database that may
/// want to be sampled to reduce the load on the database.
pub trait Sampleable {
    /// Called to set the sample rate.
    ///
    /// [`sampling::Sampling`] is used to determine if a sample should be
    /// included or excluded in sampling. It can then be applied to a
    /// [`Sampleable`] item to attach its sample rate.
    ///
    /// One can default to a sample rate of 1 when this is not called,
    /// meaning that the item was not sampled.
    fn set_sample_rate(&mut self, rate: NonZeroU64);
}
