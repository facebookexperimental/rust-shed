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

use std::net::Ipv6Addr;

use fbthrift::adapter::FromStrAdapter;

/// Adapts thrift strings as [`Ipv6Addr`]s.
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::Ipv6AddressAdapter"}
/// typedef string Ipv6Address;
///
/// struct CreateWorkflowRequest {
///   1: Ipv6Address target;
/// }
/// ```
// NOTE: Generic `T` (defaulted to `String`) is present in order to preserve
// backwards compatibility with users of the adapter passing the generic type
// parameter (`@rust.Adapter{name = "::fbthrift_adapters::Ipv6AddressAdapter<>"}`).
// Once all users have migrated to the format without the generic type parameter
// then the generic from the type alias can be removed.
// If `T` generic is removed, we will be able to remove the redundant generic from
// `FromStrAdapter` as well.
pub type Ipv6AddressAdapter<T = String> = FromStrAdapter<Ipv6Addr, T>;

#[cfg(test)]
mod string_impl {
    use fbthrift::adapter::ThriftAdapter;

    use super::*;

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
