/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#[cfg(any(fbcode_build, feature = "fb"))]
mod facebook;
#[cfg(any(fbcode_build, feature = "fb"))]
pub use facebook::is_prod;

#[cfg(not(any(fbcode_build, feature = "fb")))]
pub fn is_prod() -> bool {
    false
}

#[no_mangle]
pub extern "C" fn fb_is_prod() -> bool {
    is_prod()
}

#[no_mangle]
pub extern "C" fn fb_has_servicerouter() -> bool {
    is_prod()
}
