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

use std::net::Ipv4Addr;

use fbthrift::adapter::FromStrAdapter;

/// Adapts thrift strings as [`Ipv4Addr`]s.
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::Ipv4AddressAdapter"}
/// typedef string Ipv4Address;
///
/// struct CreateWorkflowRequest {
///   1: Ipv4Address target;
/// }
/// ```
// NOTE: Generic `T` (defaulted to `String`) is present in order to preserve
// backwards compatibility with users of the adapter passing the generic type
// parameter (`@rust.Adapter{name = "::fbthrift_adapters::Ipv4AddressAdapter<>"}`).
// Once all users have migrated to the format without the generic type parameter
// then the generic from the type alias can be removed.
// If `T` generic is removed, we will be able to remove the redundant generic from
// `FromStrAdapter` as well.
pub type Ipv4AddressAdapter<T = String> = FromStrAdapter<Ipv4Addr, T>;

#[cfg(test)]
mod string_impl {
    use fbthrift::adapter::ThriftAdapter;

    use super::*;

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
