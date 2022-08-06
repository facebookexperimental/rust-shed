/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]
#![allow(unstable_name_collisions)]

//! Crate extending functionality of [std::time]

use std::time::Duration;
use std::time::Instant;

use anyhow::Result;
use thiserror::Error;

/// Error that might be returned when requesting time e.g. in micro seconds,
/// but the result wouldn't fit into u64.
#[derive(Debug, Error)]
#[error("value too large for u64")]
pub struct OverflowError;

/// A trait implemented for [Duration] that extends the standard functionality.
pub trait DurationExt {
    /// Returns the number of whole milliseconds contained in this `Duration`.
    /// Results in an error if the resulting value would overflow a `u64`.
    fn as_millis_u64(&self) -> Result<u64>;

    /// Returns the number of whole milliseconds contained in this `Duration`.
    /// Does not check for overflow.
    fn as_millis_unchecked(&self) -> u64;

    /// Returns the number of whole microseconds contained in this `Duration`.
    /// Results in an error if the resulting value would overflow a `u64`.
    fn as_micros_u64(&self) -> Result<u64>;

    /// Returns the number of whole microseconds contained in this `Duration`.
    /// Does not check for overflow.
    fn as_micros_unchecked(&self) -> u64;

    /// Returns the number of whole nanoseconds contained in this `Duration`.
    /// Results in an error if the resulting value would overflow a `u64`.
    fn as_nanos_u64(&self) -> Result<u64>;

    /// Returns the number of whole nanoseconds contained in this `Duration`.
    /// Does not check for overflow.
    fn as_nanos_unchecked(&self) -> u64;

    /// Create a new `Duration` from the specified number of microseconds. This is exactly
    /// the same as the experimental Duration::from_micros method, and is provided here
    /// to save users from the trouble of enabling the `duration_from_micros` feature.
    fn from_micros(micros: u64) -> Self;

    /// Create a new `Duration` from the specified number of microseconds. This is exactly
    /// the same as the experimental Duration::from_nanos method, and is provided here
    /// to save users from the trouble of enabling the `duration_extras` feature.
    fn from_nanos(nanos: u64) -> Self;

    /// Return a `Duration` of length 0.
    fn zero() -> Self;

    /// Returns `true` if the duration equals `Duration::zero()`.
    fn is_zero(&self) -> bool;
}

impl DurationExt for Duration {
    fn as_millis_u64(&self) -> Result<u64> {
        self.as_millis()
            .try_into()
            .map_err(|_| OverflowError.into())
    }

    fn as_millis_unchecked(&self) -> u64 {
        self.as_secs() * 1_000 + self.subsec_millis() as u64
    }

    fn as_micros_u64(&self) -> Result<u64> {
        self.as_micros()
            .try_into()
            .map_err(|_| OverflowError.into())
    }

    fn as_micros_unchecked(&self) -> u64 {
        self.as_secs() * 1_000_000 + self.subsec_micros() as u64
    }

    fn as_nanos_u64(&self) -> Result<u64> {
        self.as_nanos().try_into().map_err(|_| OverflowError.into())
    }

    fn as_nanos_unchecked(&self) -> u64 {
        self.as_secs() * 1_000_000_000 + self.subsec_nanos() as u64
    }

    fn from_micros(micros: u64) -> Self {
        Duration::from_micros(micros)
    }

    fn from_nanos(nanos: u64) -> Self {
        Duration::from_nanos(nanos)
    }

    fn zero() -> Self {
        // Note that this is more efficient than Duration::new(0, 0) because it does not
        // need to perform range checks on the nanosecond component.
        Duration::from_secs(0)
    }

    fn is_zero(&self) -> bool {
        self == &Self::zero()
    }
}

/// A trait implemented for [Instant] that extends the standard functionality.
pub trait InstantExt {
    /// Returns the amount of time elapsed from this `Instant` to a later one.
    /// Corollary to `std::time::Instant::duration_since()`.
    fn duration_until(&self, later: Self) -> Duration;
}

impl InstantExt for Instant {
    fn duration_until(&self, later: Self) -> Duration {
        later.duration_since(*self)
    }
}

#[cfg(test)]
mod tests {
    use quickcheck::quickcheck;

    use super::*;

    quickcheck! {
        fn millis_checked(x: u64) -> bool {
            let dur = Duration::from_millis(x);
            let millis = dur.as_millis_u64()
                .expect("overflow impossible since original value was a u64");
            x == millis
        }

        fn micros_checked(x: u64) -> bool {
            let dur = Duration::from_micros(x);
            let micros = dur.as_micros_u64()
                .expect("overflow impossible since original value was a u64");
            x == micros
        }

        fn nanos_checked(x: u64) -> bool {
            let dur = Duration::from_nanos(x);
            let nanos = dur.as_nanos_u64()
                .expect("overflow impossible since original value was a u64");
            x == nanos
        }

        fn millis_unchecked(x: u64) -> bool {
            let dur = Duration::from_millis(x);
            let millis = dur.as_millis_unchecked();
            x == millis
        }

        fn micros_unchecked(x: u64) -> bool {
            let dur = Duration::from_micros(x);
            let micros = dur.as_micros_unchecked();
            x == micros
        }

        fn nanos_unchecked(x: u64) -> bool {
            let dur = Duration::from_nanos(x);
            let nanos = dur.as_nanos_unchecked();
            x == nanos
        }
    }

    #[test]
    fn overflow() {
        let big = Duration::from_secs(u64::max_value());
        let nanos = big.as_nanos_u64();
        assert!(nanos.is_err());
    }

    #[test]
    fn zero() {
        let zero = Duration::zero();
        assert!(zero.is_zero());
        assert_eq!(zero.as_secs(), 0);
        assert_eq!(zero.subsec_nanos(), 0);
    }

    #[test]
    fn duration_until() {
        let earlier = Instant::now();
        std::thread::sleep(Duration::from_millis(1));
        let later = Instant::now();

        let elapsed1 = earlier.duration_until(later);
        let elapsed2 = later.duration_since(earlier);
        assert_eq!(elapsed1, elapsed2);
    }
}
