/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Adapters that interpret thrift types as [`Uuid`]s.

use std::marker::PhantomData;

use fbthrift::adapter::ThriftAdapter;
use uuid::Uuid;

/// Adapts thrift strings and thrift bytes as [`Uuid`]s.
///
/// For more information, see implementation documentation.
pub struct UuidAdapter<T> {
    inner: PhantomData<T>,
}

/// Implementation for adapting thrift bytes.
///
/// This adapter can perform round-trip serialization and deserialization
/// without transforming data for all non-empty inputs.
///
/// Passing in an empty vector returns the [nil UUID] instead of an empty
/// vector.
///
/// [nil UUID]: https://en.wikipedia.org/wiki/Universally_unique_identifier#Nil_UUID
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::UuidAdapter"}
/// typedef binary uuid;
///
/// struct CreateWorkflowRequest {
///   1: uuid id;
/// }
/// ```
impl ThriftAdapter for UuidAdapter<Vec<u8>> {
    type StandardType = Vec<u8>;
    type AdaptedType = Uuid;

    type Error = uuid::Error;

    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
        value.as_bytes().to_vec()
    }

    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
        if value.is_empty() {
            Ok(Uuid::nil())
        } else {
            Uuid::from_slice(&value)
        }
    }
}

/// Implementation for adapting optionally hyphenated thrift strings.
///
/// Unlike the `Vec<u8>` implementation, this adapter may transform your data.
/// The adapter supports both hypenated and unhyphenated UUIDs, and will always
/// serializes as a hypenated lowercase string.
///
/// Passing in an empty string returns the [nil UUID].
///
/// [nil UUID]: https://en.wikipedia.org/wiki/Universally_unique_identifier#Nil_UUID
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::UuidAdapter"}
/// typedef string uuid;
///
/// struct CreateWorkflowRequest {
///   1: uuid id;
/// }
/// ```
impl ThriftAdapter for UuidAdapter<String> {
    type StandardType = String;
    type AdaptedType = Uuid;

    type Error = uuid::Error;

    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
        value.to_string()
    }

    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
        if value.is_empty() {
            Ok(Uuid::nil())
        } else {
            Uuid::try_parse(&value)
        }
    }
}

#[cfg(test)]
mod vec_impl {
    use super::*;

    type UuidAdapter = super::UuidAdapter<Vec<u8>>;

    #[test]
    fn round_trip_non_empty() {
        #[rustfmt::skip]
        let bytes = vec![
            0xa1, 0xa2, 0xa3, 0xa4,
            0xb1, 0xb2,
            0xc1, 0xc2,
            0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
        ];
        let adapted = UuidAdapter::from_thrift(bytes.clone()).unwrap();
        assert_eq!(UuidAdapter::to_thrift(&adapted), bytes);
    }

    #[test]
    fn empty() {
        let adapted = UuidAdapter::from_thrift(vec![]).unwrap();
        assert_eq!(adapted, Uuid::nil());
        assert_eq!(UuidAdapter::to_thrift(&adapted), vec![0; 16]);
    }

    #[test]
    fn invalid_uuid() {
        // uuids need to be 16 bytes long
        assert!(UuidAdapter::from_thrift(b"hello world".to_vec()).is_err());
        #[rustfmt::skip]
        let bytes = vec![
            0xa1, 0xa2, 0xa3, 0xa4, b'-',
            0xb1, 0xb2, b'-',
            0xc1, 0xc2, b'-',
            0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8,
        ];
        assert!(UuidAdapter::from_thrift(bytes).is_err());
        // uuids need to have valid bytes
        assert!(UuidAdapter::from_thrift(vec![0xff, 16]).is_err());
    }
}

#[cfg(test)]
mod string_impl {
    use super::*;

    type UuidAdapter = super::UuidAdapter<String>;

    fn test_with_mutation(f: impl FnOnce(String) -> String) {
        let original = "550e8400-e29b-41d4-a716-446655440000".to_owned();
        let adapted = UuidAdapter::from_thrift(f(original.clone())).unwrap();
        assert_eq!(UuidAdapter::to_thrift(&adapted), original);
    }

    #[test]
    fn unhypenated() {
        test_with_mutation(|original| original.chars().filter(|c| c.is_alphanumeric()).collect());
    }

    #[test]
    fn hyphenated() {
        test_with_mutation(|s| s);
    }

    #[test]
    fn uppercase() {
        test_with_mutation(|original| original.to_uppercase());
    }

    #[test]
    fn mixed_case() {
        test_with_mutation(|original| {
            original
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    if i % 2 == 0 {
                        c
                    } else {
                        c.to_ascii_uppercase()
                    }
                })
                .collect()
        })
    }

    #[test]
    fn empty() {
        let nil = "00000000-0000-0000-0000-000000000000".to_owned();
        let adapted = UuidAdapter::from_thrift(String::new()).unwrap();
        assert_eq!(adapted, Uuid::nil());
        assert_eq!(UuidAdapter::to_thrift(&adapted), nil);
    }

    #[test]
    fn invalid_uuid() {
        // uuids need to be 16 bytes long
        assert!(UuidAdapter::from_thrift("hello world".to_owned()).is_err());
        // partially-hyphenated uuids are not supported
        assert!(UuidAdapter::from_thrift("550e8400e29b41d4-a716-446655440000".to_owned()).is_err());
        // uuids need to have valid bytes
        assert!(UuidAdapter::from_thrift("gggggggggggggggg-gggg-gggggggggggg".to_owned()).is_err());
    }
}
