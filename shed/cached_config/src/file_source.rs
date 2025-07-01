/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use anyhow::Context;
use anyhow::Result;
use bytes::Bytes;

use crate::Entity;
use crate::ModificationTime;
use crate::Source;

#[derive(Debug)]
pub(crate) struct FileSource {
    directory: PathBuf,
    extension: Option<String>,
}

impl FileSource {
    pub(crate) fn new(directory: PathBuf, extension: impl Into<Option<String>>) -> Self {
        Self {
            directory,
            extension: extension.into(),
        }
    }
}

impl Source for FileSource {
    fn config_for_path(&self, path: &str) -> Result<Entity> {
        let path = {
            let mut path_with_extension = path.to_owned();
            if let Some(extension) = &self.extension {
                path_with_extension.push_str(extension);
            }
            self.directory.join(path_with_extension)
        };

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to open {}", path.to_string_lossy()))?;
        let version = contents.clone();

        let mod_time = fs::metadata(path)?
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)?
            .as_secs();

        Ok(Entity {
            contents: Some(Bytes::from(contents)),
            mod_time: ModificationTime::UnixTimestamp(mod_time),
            version,
        })
    }

    fn paths_to_refresh<'a>(&self, paths: &mut dyn Iterator<Item = &'a str>) -> Vec<&'a str> {
        paths.collect()
    }
}
