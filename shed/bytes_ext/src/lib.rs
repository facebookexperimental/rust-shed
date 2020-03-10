/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

//! This crate contains helpers to work with code that uses both bytes 0.4 and
//! 0.5.

#![deny(warnings, missing_docs, clippy::all, intra_doc_link_resolution_failure)]

mod collect;
mod compat;
mod convert;

pub use crate::collect::BytesCollect;
pub use crate::compat::BytesCompat;
pub use crate::convert::{copy_from_new, copy_from_old};
