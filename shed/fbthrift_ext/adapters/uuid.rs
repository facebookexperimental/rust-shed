/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::marker::PhantomData;

use fbthrift::adapter::ThriftAdapter;
use uuid::Uuid;

pub struct UuidAdapter<T> {
    inner: PhantomData<T>,
}

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
