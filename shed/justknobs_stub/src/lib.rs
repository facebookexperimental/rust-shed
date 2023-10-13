/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! This crate provides a client for accessing JustKnobs. The version on GitHub
//! is no-op for now.

use anyhow as _;
#[cfg(fbcode_build)]
pub use fb_justknobs::*;
#[cfg(not(fbcode_build))]
pub use stub::*;

#[cfg(not(fbcode_build))]
mod stub {
    use anyhow::Result;

    pub fn eval(_name: &str, _hash_val: Option<&str>, _switch_val: Option<&str>) -> Result<bool> {
        Ok(false)
    }

    pub fn get(_name: &str, _switch_val: Option<&str>) -> Result<i64> {
        Ok(0)
    }

    pub fn get_as<T>(_name: &str, _switch_val: Option<&str>) -> Result<T>
    where
        T: TryFrom<i64>,
        <T as TryFrom<i64>>::Error: std::error::Error + Send + Sync + 'static,
    {
        Ok(0.try_into()?)
    }
}
