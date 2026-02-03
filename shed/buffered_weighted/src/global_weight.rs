/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::fmt;
use std::sync::Arc;

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

/// Global weight implementation, shared between FutureQueue and FutureQueueGrouped.
pub struct GlobalWeight {
    max: usize,
    current: usize,
    observer: Option<Arc<dyn WeightObserver>>,
}

impl fmt::Debug for GlobalWeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GlobalWeight")
            .field("max", &self.max)
            .field("current", &self.current)
            .field("observer", &self.observer.as_ref().map(|_| "<observer>"))
            .finish()
    }
}

impl GlobalWeight {
    /// Create a new GlobalWeight with the given max weight.
    pub fn new(max: usize) -> Self {
        Self {
            max,
            current: 0,
            observer: None,
        }
    }

    /// Create a new GlobalWeight with the given max weight and observer.
    pub fn with_observer(max: usize, observer: Arc<dyn WeightObserver>) -> Self {
        Self {
            max,
            current: 0,
            observer: Some(observer),
        }
    }

    /// Get the max weight.
    #[inline]
    pub fn max(&self) -> usize {
        self.max
    }

    /// Get the current weight.
    #[inline]
    pub fn current(&self) -> usize {
        self.current
    }

    /// Check if there is enough space for the given weight.
    #[inline]
    pub fn has_space_for(&self, weight: usize) -> bool {
        let weight = weight.min(self.max);
        self.current <= self.max - weight
    }

    /// Add the given weight to the current weight.
    pub fn add_weight(&mut self, weight: usize) {
        let clamped_weight = weight.min(self.max);
        self.current = self.current.checked_add(clamped_weight).unwrap_or_else(|| {
            panic!(
                "future_queue_grouped: added weight {} to current {}, overflowed",
                clamped_weight, self.current,
            )
        });
        if let Some(ref observer) = self.observer {
            observer.on_weight_added(weight);
        }
    }

    /// Subtract the given weight from the current weight.
    pub fn sub_weight(&mut self, weight: usize) {
        let clamped_weight = weight.min(self.max);
        self.current = self.current.checked_sub(clamped_weight).unwrap_or_else(|| {
            panic!(
                "future_queue_grouped: subtracted weight {} from current {}, overflowed",
                clamped_weight, self.current,
            )
        });
        if let Some(ref observer) = self.observer {
            observer.on_weight_removed(weight);
        }
    }
}
