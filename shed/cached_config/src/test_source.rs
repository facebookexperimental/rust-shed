/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::anyhow;
use anyhow::Result;
use bytes::Bytes;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::Mutex;

use crate::Entity;
use crate::ModificationTime;
use crate::Source;

/// In-memory version of config source. Useful for testing
#[derive(Debug)]
pub struct TestSource {
    path_to_config: Arc<Mutex<HashMap<String, Entity>>>,
    to_refresh: Arc<Mutex<HashSet<String>>>,
}

impl Source for TestSource {
    fn config_for_path(&self, path: &str) -> Result<Entity> {
        self.path_to_config
            .lock()
            .expect("poisoned lock")
            .get(path)
            .cloned()
            .ok_or_else(|| anyhow!("Config not present for {:?}", path))
    }

    fn paths_to_refresh<'a>(&self, paths: &mut dyn Iterator<Item = &'a str>) -> Vec<&'a str> {
        let to_refresh = self.to_refresh.lock().expect("poisoned lock");
        paths.filter(|p| to_refresh.contains(*p)).collect()
    }
}

impl TestSource {
    /// Create an empty instance of `TestSource`
    pub fn new() -> Self {
        Self {
            path_to_config: Arc::new(Mutex::new(HashMap::new())),
            to_refresh: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Insert config value into the `TestSource`, overwriting existing one
    pub fn insert_config(&self, key: &str, contents: &str, mod_time: ModificationTime) {
        let mut map = self.path_to_config.lock().expect("poisoned lock");
        map.insert(
            key.to_owned(),
            Entity {
                contents: Some(Bytes::copy_from_slice(contents.as_bytes())),
                mod_time,
                version: String::new(),
            },
        );
    }

    /// Insert a new config path into a `to_refresh` set of `TestSource`
    pub fn insert_to_refresh(&self, path: String) {
        let mut to_refresh = self.to_refresh.lock().expect("poisoned lock");
        to_refresh.insert(path);
    }
}

impl Default for TestSource {
    fn default() -> Self {
        Self::new()
    }
}
