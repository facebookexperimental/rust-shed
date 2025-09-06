/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

/// Global weight implementation, shared between FutureQueue and FutureQueueGrouped.
#[derive(Debug)]
pub struct GlobalWeight {
    max: usize,
    current: usize,
}

impl GlobalWeight {
    /// Create a new GlobalWeight with the given max weight.
    pub fn new(max: usize) -> Self {
        Self { max, current: 0 }
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
        let weight = weight.min(self.max);
        self.current = self.current.checked_add(weight).unwrap_or_else(|| {
            panic!(
                "future_queue_grouped: added weight {} to current {}, overflowed",
                weight, self.current,
            )
        });
    }

    /// Subtract the given weight from the current weight.
    pub fn sub_weight(&mut self, weight: usize) {
        let weight = weight.min(self.max);
        self.current = self.current.checked_sub(weight).unwrap_or_else(|| {
            panic!(
                "future_queue_grouped: subtracted weight {} from current {}, overflowed",
                weight, self.current,
            )
        });
    }
}
