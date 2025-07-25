/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::time::Duration;

/// A fixed interval backoff strategy that always returns the same duration.
pub struct FixedInterval {
    interval: Duration,
}

impl FixedInterval {
    /// Create a new fixed interval backoff strategy.
    pub fn new(interval: Duration) -> Self {
        Self { interval }
    }
}

impl Iterator for FixedInterval {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.interval)
    }
}

/// An exponential backoff strategy that multiplies the previous duration by a base value.
pub struct ExponentialBackoff {
    base: f64,
    current: Duration,
}

impl ExponentialBackoff {
    /// Create a new exponential backoff strategy with the given base.
    pub fn new(initial_interval: Duration, base: f64) -> Self {
        Self {
            base,
            current: initial_interval,
        }
    }

    /// Create a new binary (base 2) exponential backoff strategy.
    pub fn binary(initial_interval: Duration) -> Self {
        Self::new(initial_interval, 2.0)
    }
}

impl Iterator for ExponentialBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current;
        self.current = current.mul_f64(self.base);
        Some(current)
    }
}

/// A Fibonacci backoff strategy that follows the Fibonacci sequence pattern.
pub struct FibonacciBackoff {
    current: Duration,
    next: Duration,
}

impl FibonacciBackoff {
    /// Create a new Fibonacci backoff strategy.
    pub fn new(initial_interval: Duration) -> Self {
        Self {
            current: initial_interval,
            next: initial_interval,
        }
    }
}

impl Iterator for FibonacciBackoff {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current;
        let next_next = self.current + self.next;
        self.current = self.next;
        self.next = next_next;
        Some(current)
    }
}

/// A wrapper that adds random jitter to another backoff strategy.
pub struct Jitter<B> {
    inner: B,
    jitter: Duration,
}

impl<B> Jitter<B> {
    /// Create a new jitter wrapper around another backoff strategy.
    pub fn new(inner: B, jitter: Duration) -> Self {
        Self { inner, jitter }
    }
}

impl<B: Iterator<Item = Duration>> Iterator for Jitter<B> {
    type Item = Duration;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|interval| {
            let jitter = self.jitter.mul_f64(rand::random::<f64>());
            interval + jitter
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn test_fixed_interval() {
        // Test with 100ms
        let intervals = FixedInterval::new(Duration::from_millis(100))
            .take(6)
            .collect::<Vec<_>>();
        for interval in intervals {
            assert_eq!(interval, Duration::from_millis(100));
        }

        // Test with 500ms
        let intervals = FixedInterval::new(Duration::from_millis(500))
            .take(6)
            .collect::<Vec<_>>();
        for interval in intervals {
            assert_eq!(interval, Duration::from_millis(500));
        }

        // Test with 1s
        let intervals = FixedInterval::new(Duration::from_secs(1))
            .take(6)
            .collect::<Vec<_>>();
        for interval in intervals {
            assert_eq!(interval, Duration::from_secs(1));
        }
    }

    #[test]
    fn test_exponential_backoff_binary() {
        // Test with 100ms initial interval
        let intervals = ExponentialBackoff::binary(Duration::from_millis(100))
            .take(6)
            .collect::<Vec<_>>();
        let expected = vec![
            Duration::from_millis(100),
            Duration::from_millis(200),
            Duration::from_millis(400),
            Duration::from_millis(800),
            Duration::from_millis(1600),
            Duration::from_millis(3200),
        ];
        assert_eq!(intervals, expected);

        // Test with 1s initial interval
        let intervals = ExponentialBackoff::binary(Duration::from_secs(1))
            .take(6)
            .collect::<Vec<_>>();
        let expected = vec![
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(4),
            Duration::from_secs(8),
            Duration::from_secs(16),
            Duration::from_secs(32),
        ];
        assert_eq!(intervals, expected);
    }

    #[test]
    fn test_exponential_backoff_custom_base() {
        // Test with base 3 and 100ms initial interval
        let intervals = ExponentialBackoff::new(Duration::from_millis(100), 3.0)
            .take(6)
            .collect::<Vec<_>>();
        let expected = vec![
            Duration::from_millis(100),
            Duration::from_millis(300),
            Duration::from_millis(900),
            Duration::from_millis(2700),
            Duration::from_millis(8100),
            Duration::from_millis(24300),
        ];
        assert_eq!(intervals, expected);

        // Test with base 10 and 50ms initial interval
        let intervals = ExponentialBackoff::new(Duration::from_millis(50), 10.0)
            .take(6)
            .collect::<Vec<_>>();
        let expected = vec![
            Duration::from_millis(50),
            Duration::from_millis(500),
            Duration::from_millis(5000),
            Duration::from_millis(50000),
            Duration::from_millis(500000),
            Duration::from_millis(5000000),
        ];
        assert_eq!(intervals, expected);
    }

    #[test]
    fn test_fibonacci_backoff() {
        // Test with 100ms initial interval
        let intervals = FibonacciBackoff::new(Duration::from_millis(100))
            .take(6)
            .collect::<Vec<_>>();
        let expected = vec![
            Duration::from_millis(100),
            Duration::from_millis(100),
            Duration::from_millis(200),
            Duration::from_millis(300),
            Duration::from_millis(500),
            Duration::from_millis(800),
        ];
        assert_eq!(intervals, expected);

        // Test with 1s initial interval
        let intervals = FibonacciBackoff::new(Duration::from_secs(1))
            .take(6)
            .collect::<Vec<_>>();
        let expected = vec![
            Duration::from_secs(1),
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(3),
            Duration::from_secs(5),
            Duration::from_secs(8),
        ];
        assert_eq!(intervals, expected);

        // Test with 250ms initial interval
        let intervals = FibonacciBackoff::new(Duration::from_millis(250))
            .take(6)
            .collect::<Vec<_>>();
        let expected = vec![
            Duration::from_millis(250),
            Duration::from_millis(250),
            Duration::from_millis(500),
            Duration::from_millis(750),
            Duration::from_millis(1250),
            Duration::from_millis(2000),
        ];
        assert_eq!(intervals, expected);
    }

    #[test]
    fn test_jitter() {
        // Create a fixed interval with jitter
        let fixed_with_jitter = Jitter::new(
            FixedInterval::new(Duration::from_millis(100)),
            Duration::from_millis(50),
        );
        let intervals = fixed_with_jitter.take(6).collect::<Vec<_>>();

        // Check that all intervals are within the expected range (100ms to 150ms)
        for interval in &intervals {
            assert!(
                *interval >= Duration::from_millis(100) && *interval <= Duration::from_millis(150),
                "Interval {} is outside the expected range of 100ms-150ms",
                interval.as_millis()
            );
        }

        // Check that at least some intervals are different (jitter is being applied)
        let all_same = intervals.windows(2).all(|w| w[0] == w[1]);
        assert!(
            !all_same,
            "All intervals are the same, jitter not being applied"
        );

        // Test with exponential backoff and jitter
        let exp_with_jitter = Jitter::new(
            ExponentialBackoff::binary(Duration::from_millis(100)),
            Duration::from_millis(100),
        );
        let intervals = exp_with_jitter.take(6).collect::<Vec<_>>();

        // Expected base values without jitter
        let base_values = [
            Duration::from_millis(100),
            Duration::from_millis(200),
            Duration::from_millis(400),
            Duration::from_millis(800),
            Duration::from_millis(1600),
            Duration::from_millis(3200),
        ];

        // Check that each interval is within the expected range (base to base+jitter)
        for (i, interval) in intervals.iter().enumerate() {
            let base = base_values[i];
            let max = base + Duration::from_millis(100);
            assert!(
                *interval >= base && *interval <= max,
                "Interval {} at position {} is outside the expected range of {}ms-{}ms",
                interval.as_millis(),
                i,
                base.as_millis(),
                max.as_millis()
            );
        }

        // Test with a larger jitter value
        let large_jitter = Jitter::new(
            FixedInterval::new(Duration::from_millis(100)),
            Duration::from_secs(1),
        );
        let intervals = large_jitter.take(6).collect::<Vec<_>>();

        // Check that all intervals are within the expected range (100ms to 1.1s)
        for interval in &intervals {
            assert!(
                *interval >= Duration::from_millis(100) && *interval <= Duration::from_millis(1100),
                "Interval {} is outside the expected range of 100ms-1100ms",
                interval.as_millis()
            );
        }
    }
}
