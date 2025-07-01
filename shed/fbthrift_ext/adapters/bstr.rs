/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Adapters that interpret thrift types as [`BString`]s.

use bstr::BString;
use fbthrift::adapter::NewTypeAdapter;

/// Adapts thrift strings as [`BString`]s.
pub struct BStringAdapter;

impl NewTypeAdapter for BStringAdapter {
    type StandardType = Vec<u8>;
    type AdaptedType = BString;
}
