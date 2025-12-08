/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Adapters that interpret thrift types as [`Url`]s.

use either::Either;
use fbthrift::adapter::ThriftAdapter;
use url::Url;

/// Adapts thrift strings as [`Url`]s.
///
/// For more information, see implementation documentation.
pub struct UrlAdapter;

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
/// @rust.Adapter{name = "::fbthrift_adapters::UrlAdapter"}
/// typedef string Url;
///
/// struct CreateWorkflowRequest {
///   1: Url target;
/// }
/// ```
impl ThriftAdapter for UrlAdapter {
    type StandardType = String;
    type AdaptedType = Either<Url, String>;

    type Error = core::convert::Infallible;

    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
        value.to_string()
    }

    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
        match Url::parse(&value) {
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
        let adapted = UrlAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_right());
        assert_eq!(UrlAdapter::to_thrift(&adapted), raw_address);
    }

    #[test]
    fn round_trip_domain() {
        let raw_address = "ssh://host".to_owned();
        let adapted = UrlAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_left());
        assert_eq!(UrlAdapter::to_thrift(&adapted), raw_address);

        let raw_address = "ssh://user@host:2222".to_owned();
        let adapted = UrlAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_left());
        assert_eq!(UrlAdapter::to_thrift(&adapted), raw_address);
    }

    #[test]
    fn round_trip_ipv4() {
        let raw_address = "ssh://127.0.0.1".to_owned();
        let adapted = UrlAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_left());
        assert_eq!(UrlAdapter::to_thrift(&adapted), raw_address);

        let raw_address = "ssh://user@127.0.0.1:2222".to_owned();
        let adapted = UrlAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_left());
        assert_eq!(UrlAdapter::to_thrift(&adapted), raw_address);
    }

    #[test]
    fn round_trip_ipv6() {
        let raw_address = "ssh://[::1]".to_owned();
        let adapted = UrlAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_left());
        assert_eq!(UrlAdapter::to_thrift(&adapted), raw_address);

        let raw_address = "ssh://user@[::1]:2222".to_owned();
        let adapted = UrlAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_left());
        assert_eq!(UrlAdapter::to_thrift(&adapted), raw_address);
    }

    /// Scheme is required for [`Url`].
    #[test]
    fn round_trip_invalid_no_scheme() {
        let raw_address = "host".to_owned();
        let adapted = UrlAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_right());
        assert_eq!(UrlAdapter::to_thrift(&adapted), raw_address);

        let raw_address = "127.0.0.1".to_owned();
        let adapted = UrlAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_right());
        assert_eq!(UrlAdapter::to_thrift(&adapted), raw_address);

        let raw_address = "[::1]".to_owned();
        let adapted = UrlAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_right());
        assert_eq!(UrlAdapter::to_thrift(&adapted), raw_address);

        let raw_address = "user@host:2222".to_owned();
        let adapted = UrlAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_right());
        assert_eq!(UrlAdapter::to_thrift(&adapted), raw_address);

        let raw_address = "user@127.0.0.1:2222".to_owned();
        let adapted = UrlAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_right());
        assert_eq!(UrlAdapter::to_thrift(&adapted), raw_address);

        let raw_address = "user@[::1]:2222".to_owned();
        let adapted = UrlAdapter::from_thrift(raw_address.clone()).unwrap();
        assert!(adapted.is_right());
        assert_eq!(UrlAdapter::to_thrift(&adapted), raw_address);
    }
}
