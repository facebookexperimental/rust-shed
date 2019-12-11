/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

//! Integrating Lua coroutines with Rust futures.
#![deny(warnings, missing_docs, clippy::all, intra_doc_link_resolution_failure)]

mod any_future;
mod coroutine;
#[cfg(test)]
mod tests;
mod utils;

pub use crate::any_future::AnyFuture;
pub use crate::coroutine::{LuaCoroutine, LuaCoroutineBuilder};
