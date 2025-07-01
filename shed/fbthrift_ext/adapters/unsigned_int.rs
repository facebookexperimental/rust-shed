/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Adapters that interpret Thrift signed integer types as unsigned in memory.
//! The bit patterns are unchanged, but they are intepreted as unsigned.
use std::marker::PhantomData;

use fbthrift::adapter::ThriftAdapter;

/// Adapts signed Thrift integers to unsigned Rust types
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::UnsignedIntAdapter<>"}
/// typedef i64 Uint64;
///
/// struct Parameters {
///   1: Uint64 my_param;
/// }
/// ```
pub struct UnsignedIntAdapter<T>(PhantomData<T>);

macro_rules! unsigned_int_adapter {
    ($thrift:ty, $rust:ty, $adapter:ty) => {
        impl ThriftAdapter for $adapter {
            type StandardType = $thrift;
            type AdaptedType = $rust;
            type Error = std::convert::Infallible;

            fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
                *value as _
            }

            fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
                Ok(value as _)
            }
        }
    };
}

unsigned_int_adapter!(i8, u8, UnsignedIntAdapter<i8>);
unsigned_int_adapter!(i16, u16, UnsignedIntAdapter<i16>);
unsigned_int_adapter!(i32, u32, UnsignedIntAdapter<i32>);
unsigned_int_adapter!(i64, u64, UnsignedIntAdapter<i64>);

#[cfg(target_pointer_width = "64")]
/// Adapts the platform-appropriate signed Thrift integer type to `usize`
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::UsizeAdapter"}
/// typedef i64 Usize;
///
/// struct Parameters {
///   1: Usize my_param;
/// }
/// ```
pub struct UsizeAdapter(PhantomData<i64>);

#[cfg(target_pointer_width = "64")]
unsigned_int_adapter!(i64, usize, UsizeAdapter);
