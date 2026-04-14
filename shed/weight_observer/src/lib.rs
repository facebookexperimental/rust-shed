/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

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

/// Observer wrapper that forwards `on_weight_added` but swallows
/// `on_weight_removed`. Used with `buffered_weighted` so that weight
/// is added when futures are scheduled but never removed when they
/// complete — the caller removes weight at a later point (e.g. when
/// the item is actually consumed and freed from memory).
pub struct AddOnlyObserver(pub Arc<dyn WeightObserver>);

impl WeightObserver for AddOnlyObserver {
    fn on_weight_added(&self, weight: usize) {
        self.0.on_weight_added(weight);
    }

    fn on_weight_removed(&self, _weight: usize) {
        // Intentionally swallowed. Weight will be removed by the
        // consumer when the item is no longer held in memory.
    }
}

/// RAII guard that removes tracked weight from the observer when dropped.
/// Ensures weight is cleaned up on all exit paths (success, error, panic).
pub struct WeightGuard {
    pub observer: Option<Arc<dyn WeightObserver>>,
    pub weight: usize,
}

impl Drop for WeightGuard {
    fn drop(&mut self) {
        if let Some(ref observer) = self.observer {
            if self.weight > 0 {
                observer.on_weight_removed(self.weight);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;

    use super::*;

    struct MockWeightObserver {
        total_added: AtomicUsize,
        total_removed: AtomicUsize,
    }

    impl MockWeightObserver {
        fn new_arc() -> Arc<Self> {
            Arc::new(Self {
                total_added: AtomicUsize::new(0),
                total_removed: AtomicUsize::new(0),
            })
        }

        fn net_weight(&self) -> i64 {
            self.total_added.load(Ordering::Relaxed) as i64
                - self.total_removed.load(Ordering::Relaxed) as i64
        }
    }

    impl WeightObserver for MockWeightObserver {
        fn on_weight_added(&self, weight: usize) {
            self.total_added.fetch_add(weight, Ordering::Relaxed);
        }

        fn on_weight_removed(&self, weight: usize) {
            self.total_removed.fetch_add(weight, Ordering::Relaxed);
        }
    }

    #[test]
    fn weight_guard_removes_weight_on_drop() {
        let observer = MockWeightObserver::new_arc();
        observer.on_weight_added(100);
        assert_eq!(observer.net_weight(), 100);

        {
            let _guard = WeightGuard {
                observer: Some(observer.clone()),
                weight: 100,
            };
            assert_eq!(observer.net_weight(), 100);
        }
        assert_eq!(observer.net_weight(), 0);
    }

    #[test]
    fn weight_guard_skips_zero_weight() {
        let observer = MockWeightObserver::new_arc();
        {
            let _guard = WeightGuard {
                observer: Some(observer.clone()),
                weight: 0,
            };
        }
        assert_eq!(observer.total_removed.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn weight_guard_handles_none_observer() {
        let _guard = WeightGuard {
            observer: None,
            weight: 100,
        };
    }

    #[test]
    fn add_only_observer_swallows_remove() {
        let inner = MockWeightObserver::new_arc();
        let add_only = AddOnlyObserver(inner.clone());

        add_only.on_weight_added(100);
        assert_eq!(inner.total_added.load(Ordering::Relaxed), 100);

        add_only.on_weight_removed(100);
        assert_eq!(inner.total_removed.load(Ordering::Relaxed), 0);
        assert_eq!(inner.net_weight(), 100);
    }

    #[test]
    fn weight_guard_with_add_only_observer_lifecycle() {
        let observer = MockWeightObserver::new_arc();
        let add_only = Arc::new(AddOnlyObserver(observer.clone()));

        add_only.on_weight_added(50);
        add_only.on_weight_added(75);
        add_only.on_weight_removed(50);
        add_only.on_weight_removed(75);

        assert_eq!(observer.net_weight(), 125);

        {
            let _guard = WeightGuard {
                observer: Some(observer.clone()),
                weight: 125,
            };
            assert_eq!(observer.net_weight(), 125);
        }
        assert_eq!(observer.net_weight(), 0);
    }
}
