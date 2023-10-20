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
use anyhow::Result;
#[cfg(fbcode_build)]
use fb_justknobs as default_implementation;
#[cfg(not(fbcode_build))]
use stub as default_implementation;

pub mod cached_config;

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

use cached_config::in_use as cached_config_just_knobs_in_use;

pub fn eval(name: &str, hash_val: Option<&str>, switch_val: Option<&str>) -> Result<bool> {
    if cached_config_just_knobs_in_use() {
        cached_config::eval(name, hash_val, switch_val)
    } else {
        default_implementation::eval(name, hash_val, switch_val)
    }
}

pub fn get(name: &str, switch_val: Option<&str>) -> Result<i64> {
    if cached_config_just_knobs_in_use() {
        cached_config::get(name, switch_val)
    } else {
        default_implementation::get(name, switch_val)
    }
}

pub fn get_as<T>(name: &str, switch_val: Option<&str>) -> Result<T>
where
    T: TryFrom<i64>,
    <T as TryFrom<i64>>::Error: std::error::Error + Send + Sync + 'static,
{
    if cached_config_just_knobs_in_use() {
        cached_config::get_as(name, switch_val)
    } else {
        default_implementation::get_as(name, switch_val)
    }
}
