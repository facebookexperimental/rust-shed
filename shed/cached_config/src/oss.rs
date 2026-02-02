/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use fbinit::FacebookInit;

use crate::ConfigStore;

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
        _: impl crate::IntoOptionLogger,
        _: bool,
        _: Duration,
    ) -> Result<Self> {
        fb_unimplemented!()
    }

    /// # Panics
    /// When called in non-fbcode builds
    pub fn signed_configerator(
        _: FacebookInit,
        _: impl crate::IntoOptionLogger,
        _: HashMap<String, String>,
        _: bool,
        _: Duration,
    ) -> Result<Self> {
        fb_unimplemented!()
    }

    /// # Panics
    /// When called in non-fbcode builds
    pub fn regex_signed_configerator(
        _: FacebookInit,
        _: impl crate::IntoOptionLogger,
        _: Vec<(String, String)>,
        _: bool,
        _: Duration,
    ) -> Result<Self> {
        fb_unimplemented!()
    }

    /// # Panics
    /// When called in non-fbcode builds
    pub fn materialized_configs(
        _: impl crate::IntoOptionLogger,
        _: PathBuf,
        _: impl Into<Option<Duration>>,
    ) -> Self {
        fb_unimplemented!()
    }
}
