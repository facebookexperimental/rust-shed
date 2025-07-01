/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Adapters for ensuring non-negative thrift integers.

use std::fmt;
use std::marker::PhantomData;

use fbthrift::adapter::ThriftAdapter;
use paste::paste;

/// Adapts thrift integers, ensuring they are non-negative.
///
/// The adapted type will be one of the `NonNegative` types in the
/// [`nonnegative`](crate::nonnegative) module.
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::NonNegativeAdapter<>"}
/// typedef i64 fbid;
///
/// struct GetTaskRequest {
///   1: fbid id;
/// }
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NonNegativeAdapter<T>(PhantomData<T>);

/// The error returned by [`NonNegativeAdapter`] when provided a negative value.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct NonNegativeError(pub i64);

impl fmt::Display for NonNegativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} was provided to nonnegative adapter", self.0)
    }
}

impl std::error::Error for NonNegativeError {}

macro_rules! make_nonnegative_impl {
    ($($std_type:ty),+) => {
        paste! {
            $(
                /// An integer that is known to be non-negative.
                ///
                /// This closely reflects the standard library non-zero integer
                /// variants, but lacks most trait implementations and methods.
                /// Please add them as necessary if you're missing one of them.
                #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
                pub struct [< NonNegative $std_type:upper >]($std_type);

                impl fmt::Display for [< NonNegative $std_type:upper >] {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        self.0.fmt(f)
                    }
                }

                impl [< NonNegative $std_type:upper >] {
                    /// The size of this non-negative integer type in bits.
                    /// This value is equal to u16::BITS.
                    #[doc = "The size of this non-negative integer type in bits.\n\n This value is equal to [`" $std_type "::BITS`]."]
                    pub const BITS: u32 = $std_type::BITS;

                    /// Creates a non-zero without checking whether the value is non-zero.
                    #[must_use]
                    #[inline]
                    pub const fn new_unchecked(n: $std_type) -> Self {
                        Self(n)
                    }

                    /// Creates a non-zero if the given value is not zero.
                    #[must_use]
                    #[inline]
                    pub const fn new(n: $std_type) -> Option<Self> {
                        if n < 0 {
                            None
                        } else {
                            Some(Self(n))
                        }
                    }

                    /// Returns the value as a primitive type.
                    #[inline]
                    pub const fn get(self) -> $std_type {
                        self.0
                    }
                }

                impl ThriftAdapter for NonNegativeAdapter<$std_type> {
                    type StandardType = $std_type;
                    type AdaptedType = [< NonNegative $std_type:upper >];

                    type Error = NonNegativeError;

                    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
                        value.0
                    }

                    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
                        if value < 0 {
                            return Err(NonNegativeError(value.into()));
                        }

                        Ok(Self::AdaptedType::new_unchecked(value))
                    }

                }
            )+
        }
    };
}

make_nonnegative_impl!(i8, i16, i32, i64);

#[cfg(test)]
mod tests {
    use super::*;

    type NonNegativeAdapter = super::NonNegativeAdapter<i8>;

    #[test]
    fn negative() {
        assert!(NonNegativeAdapter::from_thrift(-1).is_err());
    }

    #[test]
    fn zero() {
        let adapted = NonNegativeAdapter::from_thrift(0).unwrap();
        assert_eq!(adapted, NonNegativeI8(0));
        assert_eq!(NonNegativeAdapter::to_thrift(&adapted), 0);
    }

    #[test]
    fn positive() {
        let adapted = NonNegativeAdapter::from_thrift(1).unwrap();
        assert_eq!(adapted, NonNegativeI8(1));
        assert_eq!(NonNegativeAdapter::to_thrift(&adapted), 1);
    }
}
