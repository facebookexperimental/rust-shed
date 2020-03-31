/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, intra_doc_link_resolution_failure)]

//! This crate defines the ConfigStore which can be used to maintain a cached
//! set of configs identitied by their paths that are periodically refreshed.
//! The configs are provided by the implementors of the Source trait.

#[cfg(fbcode_build)]
mod facebook;
mod file_source;
mod handle;
mod refreshable_entities;
mod store;
#[cfg(test)]
mod tests;

pub use handle::ConfigHandle;
pub use store::ConfigStore;

use anyhow::Result;
use std::fmt::Debug;

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
    pub contents: String,
    /// Modification time of the config, e.g. file modification time
    pub mod_time: u64,
    /// Optional version of the config, together with mod_time it is used to
    /// decide if the config has changed or not
    pub version: Option<String>,
}

#[cfg(not(fbcode_build))]
mod r#impl {
    use super::*;

    use fbinit::FacebookInit;
    use slog::Logger;
    use std::{collections::HashMap, path::PathBuf, time::Duration};

    macro_rules! fb_unimplemented {
        () => {
            unimplemented!("This is implemented only for fbcode_build!")
        };
    }

    impl ConfigStore {
        /// # Panics
        /// When called in non-fbcode builds
        pub fn configerator(
            _: FacebookInit,
            _: impl Into<Option<Logger>>,
            _: Duration,
            _: Duration,
        ) -> Result<Self> {
            fb_unimplemented!()
        }

        /// # Panics
        /// When called in non-fbcode builds
        pub fn signed_configerator(
            _: FacebookInit,
            _: impl Into<Option<Logger>>,
            _: HashMap<String, String>,
            _: Duration,
            _: Duration,
        ) -> Result<Self> {
            fb_unimplemented!()
        }

        /// # Panics
        /// When called in non-fbcode builds
        pub fn materialized_configs(_: impl Into<Option<Logger>>, _: PathBuf, _: Duration) -> Self {
            fb_unimplemented!()
        }
    }
}
