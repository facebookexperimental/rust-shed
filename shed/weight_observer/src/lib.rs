/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

/// Trait for observing weight changes in buffered streams.
///
/// Implement this trait to receive callbacks when futures are scheduled
/// (weight added) or completed (weight removed) in a BufferedWeighted stream.
pub trait WeightObserver: Send + Sync {
    /// Called when a future is scheduled and its weight is added to the buffer.
    ///
    /// # Arguments
    /// * `weight` - The weight that was added (clamped to max_weight)
    fn on_weight_added(&self, weight: usize);

    /// Called when a future completes and its weight is removed from the buffer.
    ///
    /// # Arguments
    /// * `weight` - The weight that was removed (clamped to max_weight)
    fn on_weight_removed(&self, weight: usize);
}
