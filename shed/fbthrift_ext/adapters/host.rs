/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Adapters that interpret thrift types as [`Host`]s.

use either::Either;
use fbthrift::adapter::ThriftAdapter;
use url::Host;

/// Adapts thrift strings as [`Host`]s.
///
/// For more information, see implementation documentation.
pub struct HostAdapter;

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
/// @rust.Adapter{name = "::fbthrift_adapters::HostAdapter"}
/// typedef string Host;
///
/// struct CreateWorkflowRequest {
///   1: Host target;
/// }
/// ```
impl ThriftAdapter for HostAdapter {
    type StandardType = String;
    type AdaptedType = Either<Host, String>;

    type Error = core::convert::Infallible;

    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
        value.to_string()
    }

    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
        match Host::parse(&value) {
            Ok(host) => Ok(Either::Left(host)),
            Err(_) => Ok(Either::Right(value)),
        }
    }
}

#[cfg(test)]
mod string_impl {
    use super::*;

    #[test]
    fn round_trip_default() {
        let raw_address = String::default();
        let adapted = HostAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_right());
        assert_eq!(HostAdapter::to_thrift(&adapted), raw_address);
    }

    #[test]
    fn round_trip_domain() {
        let raw_address = "valid".to_owned();
        let adapted = HostAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_left());
        assert_eq!(HostAdapter::to_thrift(&adapted), raw_address);
    }

    #[test]
    fn round_trip_ipv4() {
        let raw_address = "127.0.0.1".to_owned();
        let adapted = HostAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_left());
        assert_eq!(HostAdapter::to_thrift(&adapted), raw_address);
    }

    #[test]
    fn round_trip_ipv6() {
        let raw_address = "[::1]".to_owned();
        let adapted = HostAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_left());
        assert_eq!(HostAdapter::to_thrift(&adapted), raw_address);
    }

    /// [`Host`] must adhere to domain parsing rules, so some special
    /// characters are disallowed.
    #[test]
    fn round_trip_invalid_domain() {
        let raw_address = "inv@lid".to_owned();
        let adapted = HostAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_right());
        assert_eq!(HostAdapter::to_thrift(&adapted), raw_address);
    }

    /// Port is not part of the [`Host`] segment. For that, use a full URL.
    #[test]
    fn round_trip_invalid_contains_port() {
        let raw_address = "[::1]:80".to_owned();
        let adapted = HostAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_right());
        assert_eq!(HostAdapter::to_thrift(&adapted), raw_address);
    }
}
