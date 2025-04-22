/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Adapters that interpret thrift integers as [`chrono::DateTime`]s.

use chrono::DateTime;
use chrono::Utc;
use fbthrift::adapter::ThriftAdapter;
use thiserror::Error;

/// The error returned by [`UtcTimestampAdapter`]'s [`ThriftAdapter`]
/// implementation.
///
/// [`ThriftAdapter`]: fbthrift::adapter::ThriftAdapter
#[derive(
    Error, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default
)]
#[error("{0} exceeds the supported time of a chrono::NaiveDateTime")]
pub struct OutOfRangeError(i64);

/// Adapts Thrift `i64`s as a [`DateTime<Utc>`].
///
/// This adapter interprets the `i64` as the number of non-leap seconds since
/// midnight UTC on January 1, 1970. This is commonly referred to the UNIX
/// timestamp. This adapter thus naturally does not have sub-second precision.
///
/// Note that negative numbers are valid and are interpreted as how many seconds
/// before January 1, 1970. Other langauges and implementations should be
/// careful not to intepret negative numbers as values far, far in the future
/// (e.g. don't reinterpret cast to an `uint64_t`!).
///
/// Note that this adapter is only implemented on `i64`. This is intentional for
/// multiple reasons:
///
///  1. All services should already be using `i64`s anyways for timestamps to
///     avoid the integer overflow in 2038.
///
///  2. The underlying type natively supports `i64`s.
///
/// # Errors
///
/// This adapter is limited by the range supported by [`NaiveDateTime`]. Values
/// larger than 262,000 years away from the common era are unsuppported, and
/// will always fail.
///
/// [`DateTime<Utc>`]: chrono::DateTime
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::chrono::UtcTimestampAdapter"}
/// typedef i64 utc_timestamp;
///
/// struct Entry {
///   1: utc_timestamp expiration;
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct UtcTimestampAdapter;

/// Adapts Thrift `i64`s as a [`DateTime<Utc>`].
///
/// This adapter interprets the `i64` as the number of non-leap milliseconds
/// since midnight UTC on January 1, 1970.
///
/// Note that negative numbers are valid and are interpreted as how many seconds
/// before January 1, 1970. Other langauges and implementations should be
/// careful not to intepret negative numbers as values far, far in the future
/// (e.g. don't reinterpret cast to an `uint64_t`!).
///
/// Note that this adapter is only implemented on `i64`. This is intentional for
/// multiple reasons:
///
///  1. All services should already be using `i64`s anyways for timestamps to
///     avoid the integer overflow in 2038.
///
///  2. The underlying type natively supports `i64`s.
///
/// # Errors
///
/// This adapter is limited by the range supported by [`NaiveDateTime`]. Values
/// larger than 262,000 years away from the common era are unsuppported, and
/// will always fail.
///
/// [`DateTime<Utc>`]: chrono::DateTime
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::chrono::UtcMillisecondTimestampAdapter"}
/// typedef i64 utc_timestamp_ms;
///
/// struct Entry {
///   1: utc_timestamp_ms expiration;
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct UtcMillisecondTimestampAdapter;

/// Adapts Thrift `i64`s as a [`DateTime<Utc>`].
///
/// This adapter interprets the `i64` as the number of non-leap microseconds
/// since midnight UTC on January 1, 1970.
///
/// Note that negative numbers are valid and are interpreted as how many seconds
/// before January 1, 1970. Other langauges and implementations should be
/// careful not to intepret negative numbers as values far, far in the future
/// (e.g. don't reinterpret cast to an `uint64_t`!).
///
/// Note that this adapter is only implemented on `i64`. This is intentional for
/// multiple reasons:
///
///  1. All services should already be using `i64`s anyways for timestamps to
///     avoid the integer overflow in 2038.
///
///  2. The underlying type natively supports `i64`s.
///
/// # Errors
///
/// This adapter is limited by the range supported by [`NaiveDateTime`]. Values
/// larger than 262,000 years away from the common era are unsuppported, and
/// will always fail.
///
/// [`DateTime<Utc>`]: chrono::DateTime
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::chrono::UtcMillisecondTimestampAdapter"}
/// typedef i64 utc_timestamp_us;
///
/// struct Entry {
///   1: utc_timestamp_us expiration;
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct UtcMicrosecondTimestampAdapter;

macro_rules! impl_timestamp_adapter {
    ($adapter:ident, $to_thrift_fn:expr_2021, $from_thrift_fn:expr_2021) => {
        impl ThriftAdapter for $adapter {
            type StandardType = i64;
            type AdaptedType = DateTime<Utc>;

            type Error = OutOfRangeError;

            #[allow(clippy::redundant_closure_call)]
            fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
                $to_thrift_fn(value)
            }

            #[allow(clippy::redundant_closure_call)]
            fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
                $from_thrift_fn(value).ok_or(OutOfRangeError(value))
            }
        }
    };
}

impl_timestamp_adapter!(UtcTimestampAdapter, DateTime::<Utc>::timestamp, |val| {
    DateTime::from_timestamp(val, 0)
});
impl_timestamp_adapter!(
    UtcMillisecondTimestampAdapter,
    DateTime::<Utc>::timestamp_millis,
    DateTime::from_timestamp_millis
);
impl_timestamp_adapter!(
    UtcMicrosecondTimestampAdapter,
    DateTime::<Utc>::timestamp_micros,
    DateTime::from_timestamp_micros
);

macro_rules! test_timestamp_adapter {
    ($name:ident, $adapter:ident, $one_unit_rfc3999:literal) => {
        #[cfg(test)]
        mod $name {
            use super::*;

            #[test]
            fn round_trip() {
                for i in -1..=1 {
                    assert_eq!(i, $adapter::to_thrift(&$adapter::from_thrift(i).unwrap()));
                }
            }

            #[test]
            fn negative() {
                assert!($adapter::from_thrift(-1).is_ok());
            }

            #[test]
            fn overflow() {
                assert!($adapter::from_thrift(i64::MAX).is_err());
                assert!($adapter::from_thrift(i64::MIN).is_err());
            }

            #[test]
            fn positive() {
                assert!($adapter::from_thrift(1).is_ok());
            }

            #[test]
            fn zero() {
                assert!($adapter::from_thrift(0).is_ok());
            }

            #[test]
            fn one_unit() {
                let val = $adapter::from_thrift(1).unwrap();
                assert_eq!(val.to_rfc3339(), $one_unit_rfc3999);
            }
        }
    };
}

test_timestamp_adapter!(
    utc_timestamp,
    UtcTimestampAdapter,
    "1970-01-01T00:00:01+00:00"
);
test_timestamp_adapter!(
    utc_millisecond_timestamp,
    UtcMillisecondTimestampAdapter,
    "1970-01-01T00:00:00.001+00:00"
);
test_timestamp_adapter!(
    utc_microsecond_timestamp,
    UtcMicrosecondTimestampAdapter,
    "1970-01-01T00:00:00.000001+00:00"
);
