/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Adapters that interpret thrift types as [`BString`]s.

use std::convert::Infallible;
use std::sync::Arc;

use bstr::BString;
use fbthrift::adapter::ThriftAdapter;

/// Adapts thrift strings as [`BString`]s.
pub struct ArcBStringAdapter;

impl ThriftAdapter for ArcBStringAdapter {
    type StandardType = Vec<u8>;
    type AdaptedType = Arc<BString>;
    type Error = Infallible;

    fn from_thrift(value: Self::StandardType) -> Result<Self::AdaptedType, Self::Error> {
        Ok(Arc::new(BString::new(value)))
    }

    fn to_thrift(value: &Self::AdaptedType) -> Self::StandardType {
        value.as_ref().to_vec()
    }
}
