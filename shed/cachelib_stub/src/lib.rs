/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

#![deny(warnings)]

#[cfg(not(fbcode_build))]
mod oss;

#[cfg(fbcode_build)]
pub use fb_cachelib::*;

#[cfg(not(fbcode_build))]
pub use crate::oss::dummy_cache as abomonation_cache;
#[cfg(not(fbcode_build))]
pub use crate::oss::dummy_cache as bincode_cache;
#[cfg(not(fbcode_build))]
pub use crate::oss::lrucache::*;
