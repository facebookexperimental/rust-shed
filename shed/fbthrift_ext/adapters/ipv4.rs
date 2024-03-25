/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Adapters that interpret thrift types as [`Ipv4Addr`]s.

use std::marker::PhantomData;
use std::net::AddrParseError;
use std::net::Ipv4Addr;
use std::str::FromStr;

use fbthrift::adapter::ThriftAdapter;

/// Adapts thrift strings as [`Ipv4Addr`]s.
///
/// For more information, see implementation documentation.
pub struct Ipv4AddressAdapter<T> {
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
/// @rust.Adapter{name = "::fbthrift_adapters::Ipv4AddressAdapter<>"}
/// typedef string Ipv4Address;
///
/// struct CreateWorkflowRequest {
///   1: Ipv4Address target;
/// }
/// ```
impl ThriftAdapter for Ipv4AddressAdapter<String> {
    type StandardType = String;
    type AdaptedType = Ipv4Addr;

    type Error = AddrParseError;

    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
        value.to_string()
    }

    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
        Ipv4Addr::from_str(&value)
    }
}

#[cfg(test)]
mod string_impl {
    use super::*;

    type Ipv4AddressAdapter = super::Ipv4AddressAdapter<String>;

    #[test]
    fn round_trip() {
        let raw_address = "127.0.0.1".to_owned();
        let adapted = Ipv4AddressAdapter::from_thrift(raw_address.clone()).unwrap();
        assert_eq!(Ipv4AddressAdapter::to_thrift(&adapted), raw_address);
    }

    #[test]
    fn invalid_ip() {
        assert!(Ipv4AddressAdapter::from_thrift("not_an_ip_address".to_owned()).is_err());
        // std::net::Ipv4Addr does not allow leading zeros.
        assert!(Ipv4AddressAdapter::from_thrift("01.01.01.01".to_owned()).is_err());
    }
}
