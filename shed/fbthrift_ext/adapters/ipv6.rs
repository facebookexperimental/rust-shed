/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Adapters that interpret thrift types as [`Ipv6Addr`]s.

use std::marker::PhantomData;
use std::net::AddrParseError;
use std::net::Ipv6Addr;
use std::str::FromStr;

use fbthrift::adapter::ThriftAdapter;

/// Adapts thrift strings as [`Ipv6Addr`]s.
///
/// For more information, see implementation documentation.
pub struct Ipv6AddressAdapter<T> {
    inner: PhantomData<T>,
}

/// Implementation for adapting a thrift string.
///
/// This adapter can perform round-trip serialization and deserialization
/// without transforming data for all non-empty inputs.
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::Ipv6AddressAdapter<>"}
/// typedef string Ipv6Address;
///
/// struct CreateWorkflowRequest {
///   1: Ipv6Address target;
/// }
/// ```
impl ThriftAdapter for Ipv6AddressAdapter<String> {
    type StandardType = String;
    type AdaptedType = Ipv6Addr;

    type Error = AddrParseError;

    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
        value.to_string()
    }

    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
        Ipv6Addr::from_str(&value)
    }
}

#[cfg(test)]
mod string_impl {
    use super::*;

    type Ipv6AddressAdapter = super::Ipv6AddressAdapter<String>;

    #[test]
    fn round_trip() {
        let raw_address = "::1".to_owned();
        let adapted = Ipv6AddressAdapter::from_thrift(raw_address.clone()).unwrap();
        assert_eq!(Ipv6AddressAdapter::to_thrift(&adapted), raw_address);
    }

    #[test]
    fn invalid_ip() {
        assert!(Ipv6AddressAdapter::from_thrift("not_an_ip_address".to_owned()).is_err());
        assert!(Ipv6AddressAdapter::from_thrift(":::::1".to_owned()).is_err());
    }
}
