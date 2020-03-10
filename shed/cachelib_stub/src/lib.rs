/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

#![deny(warnings)]

#[cfg(not(fbcode_build))]
mod abomonation_future_cache;
#[cfg(not(fbcode_build))]
mod lrucache;

#[cfg(not(fbcode_build))]
pub use crate::abomonation_future_cache::*;
#[cfg(not(fbcode_build))]
pub use crate::lrucache::*;

// export Abomonation so that users of this crate don't need to add abomination as dependency
#[cfg(not(fbcode_build))]
pub use abomonation::Abomonation;

#[cfg(fbcode_build)]
pub use cachelib::*;
