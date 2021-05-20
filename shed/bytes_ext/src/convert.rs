/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use bytes::Bytes as BytesNew;
use bytes_old::Bytes as BytesOld;

/// Create a 0.4 copy of 1.x bytes
pub fn copy_from_new(bytes: BytesNew) -> BytesOld {
    BytesOld::from(bytes.as_ref())
}

/// Create a 1.x copy of 0.4 bytes
pub fn copy_from_old(bytes: BytesOld) -> BytesNew {
    BytesNew::copy_from_slice(bytes.as_ref())
}
