/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Adapters that interpret thrift integers/floats as [`Duration`]s.
//!
//! There are two flavors of integer adapters in this module: saturating
//! adapters and wrapping adapters. Saturating adapters will serialize an
//! oversized integer as the largest value of the thrift type that it adapts,
//! while wrapping adapters truncate the value instead.
//!
//! For each flavor, there are multiple variants that describe the unit of time
//! that the adapter deserializes and serializes the value as. For example,
//! [`SaturatingMillisecondAdapter`] will always serialize any [`Duration`] in
//! milliseconds and always will deserialize fields as if they were
//! milliseconds.
//!
//! This module intentionally does not prefer wrapping or saturating adapters.

use std::marker::PhantomData;
use std::num::TryFromIntError;
use std::time::Duration;
use std::time::TryFromFloatSecsError;

use fbthrift::adapter::ThriftAdapter;
use paste::paste;

macro_rules! make_duration {
    ($granularity:ident, $accessor:ident, $($std_type:ty),+) => {
        paste! {
            #[doc =
"Saturating " $granularity:lower "s adapter for thrift integers.

This adapter supports all integer types including the byte type and interprets
the value as " $granularity:lower "s. It does not support negative values, and
will fail deserializing negative values.

When serializing values back into the thrift integer type, if the value does not
fit into thrift integer, the largest value the thrift integer supports will be
used instead. If wrapping the value on serialization is preferred instead, see
[`Wrapping" $granularity "Adapter`].

For other adapters, see the [`duration`](crate::duration) module.

# Examples

```thrift
include \"thrift/annotation/rust.thrift\";

@rust.Adapter{name = \"::fbthrift_adapters::Saturating" $granularity "Adapter\"}
typedef i64 timeout;

struct CreateWorkflowRequest {
  1: timeout id;
}
```
"]
            pub struct [< Saturating $granularity Adapter >]<T>(PhantomData<T>);

            $(
                impl ThriftAdapter for [< Saturating $granularity Adapter >]<$std_type> {
                    type StandardType = $std_type;
                    type AdaptedType = Duration;

                    type Error = TryFromIntError;

                    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
                        // Duration::as_* always returns an unsigned type, so if the
                        // output type can't fit into the standard type, we can
                        // assume it's because it's too large.
                        Self::StandardType::try_from(Duration::[< as_ $accessor >](value)).unwrap_or($std_type::MAX)
                    }

                    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
                        Ok(Duration::[< from_ $accessor >](u64::try_from(value)?))
                    }
                }
            )+

            #[doc =
"Wrapping " $granularity:lower "s adapter for thrift integers.

This adapter supports all integer types including the byte type and interprets
the value as " $granularity:lower "s. It does not support negative values, and
will fail deserializing negative values.

When serializing values back into the thrift integer type, if the value does not
fit into thrift integer, it will silently wrap the value instead. If saturating
the value on serialization is preferred instead, see
[`Saturating" $granularity "Adapter`].

For other adapters, see the [`duration`](crate::duration) module.

# Examples

```thrift
include \"thrift/annotation/rust.thrift\";

@rust.Adapter{name = \"::fbthrift_adapters::Wrapping" $granularity "Adapter\"}
typedef i64 timeout;

struct CreateWorkflowRequest {
  1: timeout id;
}
```
"]
            pub struct [< Wrapping $granularity Adapter >]<T>(PhantomData<T>);

            $(
                impl ThriftAdapter for [< Wrapping $granularity Adapter >]<$std_type> {
                    type StandardType = $std_type;
                    type AdaptedType = Duration;

                    type Error = TryFromIntError;

                    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
                        Duration::[< as_ $accessor >](value) as Self::StandardType
                    }

                    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
                        Ok(Duration::[< from_ $accessor >](u64::try_from(value)?))
                    }
                }
            )+
        }
    };
}

make_duration!(Second, secs, i8, i16, i32, i64);

make_duration!(Millisecond, millis, i8, i16, i32, i64);

make_duration!(Microsecond, micros, i8, i16, i32, i64);

make_duration!(Nanosecond, nanos, i8, i16, i32, i64);

macro_rules! make_floating_duration {
    ($granularity:ident, $accessor:ident, $($std_type:ty),+) => {
        paste! {
            #[doc =
"Floating point " $granularity:lower "s adapter for thrift floats.

This adapter supports all floating point types and interprets the value as "
$granularity:lower "s. It does not support negative values, and will fail
deserializing negative values.

For other adapters, see the [`duration`](crate::duration) module.

# Examples

```thrift
include \"thrift/annotation/rust.thrift\";

@rust.Adapter{name = \"::fbthrift_adapters::FloatingSecondAdapter\"}
typedef f64 timeout;

struct CreateWorkflowRequest {
  1: timeout id;
}
```
"]
            pub struct [< Floating $granularity Adapter >]<T>(PhantomData<T>);

            $(
                impl ThriftAdapter for [< Floating $granularity Adapter >]<$std_type> {
                    type StandardType = $std_type;
                    type AdaptedType = Duration;

                    type Error = [< TryFromFloat $accessor:camel Error >];

                    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
                        Duration::[< as_ $accessor _ $std_type >](value)
                    }

                    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
                        Duration::[< try_from_ $accessor _ $std_type >](value)
                    }
                }
            )+
        }
    }
}

make_floating_duration!(Second, secs, f32, f64);

#[cfg(test)]
mod saturating {
    use std::time::Duration;

    use super::*;

    #[test]
    fn overflow_behavior() {
        assert_eq!(
            SaturatingSecondAdapter::<i8>::to_thrift(&Duration::from_secs(256)),
            127
        );
    }

    #[test]
    fn negative() {
        assert!(SaturatingSecondAdapter::<i8>::from_thrift(-1).is_err());
    }
}

#[cfg(test)]
mod wrapping {
    use std::time::Duration;

    use super::*;

    #[test]
    fn overflow_behavior() {
        assert_eq!(
            WrappingSecondAdapter::<i8>::to_thrift(&Duration::from_secs(256)),
            0
        );
        assert_eq!(
            WrappingSecondAdapter::<i8>::to_thrift(&Duration::from_secs(128)),
            -128
        );
    }

    #[test]
    fn negative() {
        assert!(WrappingSecondAdapter::<i8>::from_thrift(-1).is_err());
    }
}

#[cfg(test)]
mod floating {
    use super::*;

    #[test]
    fn negative() {
        assert!(FloatingSecondAdapter::<f32>::from_thrift(-1.23).is_err());
        assert!(FloatingSecondAdapter::<f32>::from_thrift(f32::NEG_INFINITY).is_err());
    }

    #[test]
    fn overflow() {
        assert!(FloatingSecondAdapter::<f64>::from_thrift(1e100).is_err());
        assert!(FloatingSecondAdapter::<f32>::from_thrift(f32::INFINITY).is_err());
    }

    #[test]
    fn nan() {
        assert!(FloatingSecondAdapter::<f32>::from_thrift(f32::NAN).is_err());
    }
}
