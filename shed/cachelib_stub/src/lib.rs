/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings)]

#[cfg(fbcode_build)]
mod _unused {
    // used in oss
    use abomonation as _;
    use anyhow as _;
    use bytes as _;
    use futures_ext as _;
}

#[cfg(not(fbcode_build))]
mod oss;

// export Abomonation so that users of this crate don't need to add abomination as dependency
#[cfg(not(fbcode_build))]
pub use abomonation::Abomonation;
#[cfg(fbcode_build)]
pub use cachelib::*;

#[cfg(not(fbcode_build))]
pub use crate::oss::abomonation_future_cache::*;
#[cfg(not(fbcode_build))]
pub use crate::oss::lrucache::*;
