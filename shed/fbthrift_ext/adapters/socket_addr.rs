/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Adapters that interpret thrift types as [`SocketAddr`]s.

use std::marker::PhantomData;
use std::net::AddrParseError;
use std::net::SocketAddr;
use std::str::FromStr;

use fbthrift::adapter::ThriftAdapter;

/// Adapts thrift strings as [`SocketAddr`]s.
///
/// For more information, see implementation documentation.
pub struct SocketAddrAdapter<T> {
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
/// @rust.Adapter{name = "::fbthrift_adapters::SocketAddrAdapter<>"}
/// typedef string SocketAddr;
///
/// struct CreateWorkflowRequest {
///   1: SocketAddr target;
/// }
/// ```
impl ThriftAdapter for SocketAddrAdapter<String> {
    type StandardType = String;
    type AdaptedType = SocketAddr;

    type Error = AddrParseError;

    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
        value.to_string()
    }

    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
        SocketAddr::from_str(&value)
    }
}

#[cfg(test)]
mod string_impl {
    use super::*;

    type SocketAddrAdapter = super::SocketAddrAdapter<String>;

    #[test]
    fn round_trip_ipv4() {
        let raw_address = "127.0.0.1:8080".to_owned();
        let adapted = SocketAddrAdapter::from_thrift(raw_address.clone()).unwrap();
        assert_eq!(SocketAddrAdapter::to_thrift(&adapted), raw_address);
    }

    #[test]
    fn round_trip_ipv6() {
        let raw_address = "[::1]:8080".to_owned();
        let adapted = SocketAddrAdapter::from_thrift(raw_address.clone()).unwrap();
        assert_eq!(SocketAddrAdapter::to_thrift(&adapted), raw_address);
    }

    #[test]
    fn invalid_ip() {
        assert!(SocketAddrAdapter::from_thrift("not_an_ip_address".to_owned()).is_err());
        // std::net::Ipv4Addr does not allow leading zeros.
        assert!(SocketAddrAdapter::from_thrift("01.01.01.01:8080".to_owned()).is_err());
        // std::net::SocketAddr must wrap IPv6 in square brackets, otherwise port would be ambiguous
        assert!(SocketAddrAdapter::from_thrift("0:0:0:0:0:0:0:1:8080".to_owned()).is_err());
    }
}
