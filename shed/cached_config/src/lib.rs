/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]

//! This crate defines the ConfigStore which can be used to maintain a cached
//! set of configs identitied by their paths that are periodically refreshed.
//! The configs are provided by the implementors of the Source trait.

#[cfg(fbcode_build)]
mod facebook;
mod file_source;
mod handle;
#[cfg(not(fbcode_build))]
mod oss;
mod refreshable_entities;
mod store;
mod test_source;
#[cfg(test)]
mod tests;

use std::fmt::Debug;

use anyhow::Result;
use bytes::Bytes;
use chrono::NaiveDateTime;
pub use handle::ConfigHandle;
pub use store::ConfigStore;
pub use test_source::TestSource;

/// Trait to be implemented by sources of configuration that the `ConfigStore`
/// will use
pub trait Source: Debug {
    /// For a given path identifying the config return it's content
    fn config_for_path(&self, path: &str) -> Result<Entity>;
    /// Given a list of paths the client is interested in, return the ones that
    /// should be refreshed since the client last asked for them.
    fn paths_to_refresh<'a>(&self, paths: &mut dyn Iterator<Item = &'a str>) -> Vec<&'a str>;
}

/// Represents a configuration Entity e.g. a JSON blob
#[derive(Clone, Debug)]
pub struct Entity {
    /// Content of the config
    pub contents: Option<Bytes>,
    /// Modification time of the config, e.g. file modification time
    pub mod_time: ModificationTime,
    /// Optional version of the config, together with mod_time it is used to
    /// decide if the config has changed or not
    pub version: String,
}

/// Represents the last modification time of the given config.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ModificationTime {
    /// In some cases, the last modification time is not possible to determine.
    Unset,
    /// Number of non-leap seconds since January 1, 1970 0:00:00 UTC.
    UnixTimestamp(u64),
    /// ISO 8601 time without timezone.
    DateTime(NaiveDateTime),
}
