/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Adapters that interpret thrift types as [`Ipv4Addr`]s.

use std::marker::PhantomData;
use std::sync::Arc;

use fbthrift::adapter::ThriftAdapter;

/// Adapts a type as [`Arc`]s.
///
/// For more information, see implementation documentation.
///
/// # How is this different from `@rust.Arc` annotation?
/// Annotations are only possible for fields, but if we are interested
/// in having a map/list/set value to be an `Arc`, we need to wrap it
/// into an new thrift type such that we can annotate it.
/// Via the adapter we remove the need of defining a new thrift type
/// which could be not relevant for other languages.
pub struct ArcAdapter<T> {
    inner: PhantomData<T>,
}

/// Implementation to wrap a type into Arc.
///
/// This adapter can perform round-trip serialization and deserialization
/// without transforming data for all non-empty inputs.
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::ArcAdapter<>"}
/// typedef string ArcString;
///
/// struct CreateWorkflowRequest {
///   1: ArcString target;
/// }
/// ```
impl<T: Clone + std::fmt::Debug + PartialEq + Send + Sync> ThriftAdapter for ArcAdapter<T> {
    type StandardType = T;
    type AdaptedType = Arc<T>;

    type Error = std::convert::Infallible;

    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
        value.as_ref().clone()
    }

    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
        Ok(value.into())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn round_trip_string() {
        let value = "127.0.0.1".to_owned();
        let adapted: Arc<String> = ArcAdapter::from_thrift(value.clone()).unwrap();
        let thrift: String = ArcAdapter::to_thrift(&adapted);
        assert_eq!(thrift, value);
    }

    #[test]
    fn round_trip_integer() {
        let value: u32 = 123;
        let adapted: Arc<u32> = ArcAdapter::from_thrift(value).unwrap();
        let thrift: u32 = ArcAdapter::to_thrift(&adapted);
        assert_eq!(thrift, value);
    }
}
