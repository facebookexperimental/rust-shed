/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings)]
#![cfg_attr(not(fbcode_build), feature(never_type))]

#[cfg(fbcode_build)]
use fbinit as _; // used in oss
#[cfg(fbcode_build)]
use services_common as _; // used in oss

#[cfg(not(fbcode_build))]
mod oss;

#[cfg(fbcode_build)]
pub use services::*;

#[cfg(not(fbcode_build))]
pub use crate::oss::*;
