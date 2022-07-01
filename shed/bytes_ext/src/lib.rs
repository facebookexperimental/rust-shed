/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! This crate contains helpers to work with code that uses both bytes 0.4 and
//! 0.5.

#![deny(warnings, missing_docs, clippy::all)]

mod collect;
mod compat;
mod convert;

pub use crate::collect::BytesCollect;
pub use crate::compat::BytesCompat;
pub use crate::convert::copy_from_new;
pub use crate::convert::copy_from_old;
