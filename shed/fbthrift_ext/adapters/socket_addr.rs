/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Adapters that interpret thrift types as [`SocketAddr`]s.

use std::net::SocketAddr;

use fbthrift::adapter::FromStrAdapter;

/// Adapts thrift strings as [`SocketAddr`]s.
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::SocketAddrAdapter"}
/// typedef string SocketAddr;
///
/// struct CreateWorkflowRequest {
///   1: SocketAddr target;
/// }
/// ```
pub type SocketAddrAdapter = FromStrAdapter<SocketAddr>;

#[cfg(test)]
mod string_impl {
    use fbthrift::adapter::ThriftAdapter;

    use super::*;

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
