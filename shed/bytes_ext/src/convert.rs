/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use bytes::Bytes as BytesNew;
use bytes_old::Bytes as BytesOld;

pub fn copy_from_new(bytes: BytesNew) -> BytesOld {
    BytesOld::from(bytes.as_ref())
}

pub fn copy_from_old(bytes: BytesOld) -> BytesNew {
    BytesNew::copy_from_slice(bytes.as_ref())
}
