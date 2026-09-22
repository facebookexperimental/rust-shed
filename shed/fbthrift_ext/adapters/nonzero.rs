/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Adapters for ensuring non-zero thrift integers.
//!
//! Two variants, both parameterised by the thrift type so `<>` works for either:
//!
//! * [`NonZeroAdapter`] keeps the sign, adapting to `std::num::NonZeroI*`. Only zero is rejected.
//! * [`NonZeroUnsignedAdapter`] adapts to `std::num::NonZeroU*`. Zero and negatives are rejected,
//!   which is usually what a count wants.

use std::fmt;
use std::marker::PhantomData;

use fbthrift::adapter::ThriftAdapter;
use fbthrift::metadata::ThriftAnnotations;
use paste::paste;

use crate::field::AdaptedField;
use crate::field::DisplayField;

/// Adapts thrift integers as the signed [`std::num`] `NonZero` of the same width.
///
/// Only zero is rejected: a negative is non-zero and is accepted. Use
/// [`NonZeroUnsignedAdapter`] when the value is a count and negatives are meaningless.
///
/// Unlike [`NonNegativeAdapter`](crate::nonnegative::NonNegativeAdapter) — which has to define its
/// own types, as the standard library has no non-negative integers — this defines no types, so
/// callers get the full standard API and the niche optimisation.
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::NonZeroAdapter<>"}
/// typedef i64 temperature_delta;
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NonZeroAdapter<T>(PhantomData<T>);

/// Adapts thrift integers as the unsigned [`std::num`] `NonZero` of the same width.
///
/// Rejects zero and every negative, so the adapted type carries "this is a positive count" rather
/// than the caller having to check.
///
/// Thrift has no unsigned integers, so the value travels as the same-width signed type and the
/// representable range is `1..=i32::MAX` for `NonZeroU32`, and so on. That is inherent: the upper
/// half of the unsigned range has no wire encoding here.
/// [`UnsignedIntAdapter`](crate::unsigned_int::UnsignedIntAdapter) covers the full range instead,
/// by reinterpreting the bits — at the cost of turning negatives into very large positives rather
/// than rejecting them.
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::NonZeroUnsignedAdapter<>"}
/// typedef i32 shard_count;
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NonZeroUnsignedAdapter<T>(PhantomData<T>);

/// The error returned by [`NonZeroAdapter`] and [`NonZeroUnsignedAdapter`].
///
/// [`NonZeroAdapter`] only ever rejects zero, so its value is not news. [`NonZeroUnsignedAdapter`]
/// rejects any non-positive value, so the rejected value is reported either way.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct NonZeroError {
    /// The rejected value, widened to `i64`.
    pub value: i64,
    /// True when the rejection came from [`NonZeroAdapter`] (the signed variant, which only rejects
    /// zero); false when it came from [`NonZeroUnsignedAdapter`] (which also rejects negatives). Lets
    /// the message say what was expected.
    pub signed: bool,
    /// The struct field this came from, when the adapter was applied to a field rather than to a
    /// bare typedef.
    pub field: Option<AdaptedField>,
}

impl NonZeroError {
    fn signed(value: i64, field: Option<AdaptedField>) -> Self {
        Self {
            value,
            signed: true,
            field,
        }
    }

    fn unsigned(value: i64, field: Option<AdaptedField>) -> Self {
        Self {
            value,
            signed: false,
            field,
        }
    }
}

impl fmt::Display for NonZeroError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            value,
            signed,
            field,
        } = self;
        let expected = if *signed {
            "a non-zero value"
        } else {
            "a positive value"
        };
        write!(
            f,
            "nonzero adapter{} expected {expected}, got {value}",
            DisplayField(*field)
        )
    }
}

impl std::error::Error for NonZeroError {}

/// Signed impls: parameterised by the thrift type alone.
macro_rules! make_nonzero_signed_impl {
    ($($std_type:ty),+) => {
        paste! {
            $(
                impl ThriftAdapter for NonZeroAdapter<$std_type> {
                    type StandardType = $std_type;
                    type AdaptedType = ::std::num::[< NonZero $std_type:upper >];

                    type Error = NonZeroError;

                    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
                        value.get()
                    }

                    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
                        Self::AdaptedType::new(value)
                            .ok_or(NonZeroError::signed(value.into(), None))
                    }

                    fn from_thrift_field<T: ThriftAnnotations>(
                        value: Self::StandardType,
                        field_id: i16,
                    ) -> Result<Self::AdaptedType, Self::Error> {
                        Self::AdaptedType::new(value).ok_or(NonZeroError::signed(
                            value.into(),
                            Some(AdaptedField::of::<T>(field_id)),
                        ))
                    }
                }
            )+
        }
    };
}

/// Unsigned impls: the thrift type and the unsigned type it adapts to. The pairing is only
/// because `u8` cannot be derived from `i8` in a macro, not because the two variants are
/// related -- each list below stands on its own.
macro_rules! make_nonzero_unsigned_impl {
    ($(($std_type:ty, $unsigned:ty)),+) => {
        paste! {
            $(
                impl ThriftAdapter for NonZeroUnsignedAdapter<$std_type> {
                    type StandardType = $std_type;
                    type AdaptedType = ::std::num::[< NonZero $unsigned:upper >];

                    type Error = NonZeroError;

                    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
                        // Only reachable for a value built in Rust rather than read off the wire:
                        // `from_thrift` cannot produce anything above the signed maximum.
                        <$std_type>::try_from(value.get()).unwrap_or(<$std_type>::MAX)
                    }

                    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
                        <$unsigned>::try_from(value)
                            .ok()
                            .and_then(Self::AdaptedType::new)
                            .ok_or(NonZeroError::unsigned(value.into(), None))
                    }

                    fn from_thrift_field<T: ThriftAnnotations>(
                        value: Self::StandardType,
                        field_id: i16,
                    ) -> Result<Self::AdaptedType, Self::Error> {
                        <$unsigned>::try_from(value)
                            .ok()
                            .and_then(Self::AdaptedType::new)
                            .ok_or(NonZeroError::unsigned(
                                value.into(),
                                Some(AdaptedField::of::<T>(field_id)),
                            ))
                    }
                }
            )+
        }
    };
}

make_nonzero_signed_impl!(i8, i16, i32, i64);
make_nonzero_unsigned_impl!((i8, u8), (i16, u16), (i32, u32), (i64, u64));

#[cfg(test)]
mod tests {
    use std::num::NonZeroI8;
    use std::num::NonZeroU8;

    use super::*;

    type Signed = NonZeroAdapter<i8>;
    type Unsigned = NonZeroUnsignedAdapter<i8>;

    /// Stands in for a generated thrift struct so `from_thrift_field` can be exercised.
    struct DummyStruct;

    impl ThriftAnnotations for DummyStruct {}

    #[test]
    fn signed_rejects_only_zero() {
        let error = Signed::from_thrift(0).expect_err("0 must be rejected");
        assert_eq!(error.value, 0);
        assert_eq!(
            error.to_string(),
            "nonzero adapter expected a non-zero value, got 0"
        );
    }

    #[test]
    fn signed_accepts_negative() {
        let adapted = Signed::from_thrift(-1).expect("-1 is non-zero");
        assert_eq!(adapted, NonZeroI8::new(-1).expect("-1 is non-zero"));
        assert_eq!(Signed::to_thrift(&adapted), -1);
    }

    #[test]
    fn signed_round_trips_bounds() {
        for value in [i8::MIN, -1, 1, i8::MAX] {
            let adapted = Signed::from_thrift(value).expect("value is non-zero");
            assert_eq!(
                Signed::to_thrift(&adapted),
                value,
                "round trip must preserve {value}"
            );
        }
    }

    #[test]
    fn unsigned_rejects_zero_and_negative() {
        assert_eq!(
            Unsigned::from_thrift(0)
                .expect_err("0 must be rejected")
                .value,
            0
        );
        let error = Unsigned::from_thrift(-7).expect_err("-7 must be rejected");
        assert_eq!(
            error.to_string(),
            "nonzero adapter expected a positive value, got -7",
            "the unsigned variant must say it wanted a positive, not merely a non-zero"
        );
    }

    #[test]
    fn unsigned_round_trips_bounds() {
        for value in [1_i8, i8::MAX] {
            let adapted = Unsigned::from_thrift(value).expect("value is positive");
            assert_eq!(
                Unsigned::to_thrift(&adapted),
                value,
                "round trip must preserve {value}"
            );
        }
    }

    #[test]
    fn unsigned_above_signed_max_is_not_representable() {
        // Constructible in Rust, but it has no wire encoding in an i8 field; saturating is the
        // only total option and the doc comment says so.
        let too_large = NonZeroU8::new(u8::MAX).expect("u8::MAX is non-zero");
        assert_eq!(Unsigned::to_thrift(&too_large), i8::MAX);
    }

    #[test]
    fn field_context_names_the_field() {
        let error = Signed::from_thrift_field::<DummyStruct>(0, 7).expect_err("0 must be rejected");
        let field = error.field.expect("field context must be recorded");
        assert_eq!(field.field_id, 7);
        assert!(
            error.to_string().contains("field 7"),
            "message should name the field, got {error}"
        );

        let error =
            Unsigned::from_thrift_field::<DummyStruct>(-1, 3).expect_err("-1 must be rejected");
        assert_eq!(
            error
                .field
                .expect("field context must be recorded")
                .field_id,
            3
        );
    }
}
