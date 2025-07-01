/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Adapters that interpret thrift floating point types as [`OrderedFloat`]
//!
//! [`OrderedFloat`]: ordered_float::OrderedFloat

use std::marker::PhantomData;

use fbthrift::adapter::NewTypeAdapter;
use ordered_float::OrderedFloat;

/// Adapts thrift floats as OrderedFloat
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::OrderedFloatAdapter<>"}
/// typedef double Double;
///
/// struct Parameters {
///   1: Double my_param;
/// } (rust.ord)
/// ```
///
/// [`OrderedFloat`]: ordered_float::OrderedFloat
pub struct OrderedFloatAdapter<T>(PhantomData<T>);

impl<T: ordered_float::Float> NewTypeAdapter for OrderedFloatAdapter<T> {
    type StandardType = T;
    type AdaptedType = OrderedFloat<T>;
}
