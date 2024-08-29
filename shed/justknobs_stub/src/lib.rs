/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! This crate provides a client for accessing JustKnobs and takes care of isolating the test setup
//! from production configs. There are mulitple implementations that this module will choose from:
//!  * production, for Meta-internal build it's the configerator-based jk impl.  For open-source
//!    builds it'll be a stub that always returns false.
//!  * cached-config, which will work with any config source that cached_config crate can work with.
//!    Can be used for integration tests where the config can be read from on-disk JSON file and
//!    fully isolated from prod setup.  Used after being initialized with
//!    init_cached_config_just_knobs/init_cached_config_just_knobs_worker.
//!  * thread-local-in-memory, which is useful for testing. It allows to override justknobs within a
//!    test without affecting other tests. Used always when cfg(test) is true.

use anyhow::Result;
use cached_config::CachedConfigJustKnobs;
#[cfg(fbcode_build)]
use fb_justknobs as prod_implementation;
#[cfg(not(fbcode_build))]
use JustKnobsStub as prod_implementation;

pub mod cached_config;
mod thread_local_in_memory;
pub use cached_config::init_just_knobs as init_cached_config_just_knobs;
pub use cached_config::init_just_knobs_worker as init_cached_config_just_knobs_worker;
use thread_local_in_memory::ThreadLocalInMemoryJustKnobsImpl;

/// Those should be only used in tests.
pub mod test_helpers {
    pub use crate::thread_local_in_memory::override_just_knobs;
    pub use crate::thread_local_in_memory::with_just_knobs;
    pub use crate::thread_local_in_memory::with_just_knobs_async;
    pub use crate::thread_local_in_memory::JustKnobsInMemory;
    pub use crate::thread_local_in_memory::KnobVal;
}

/// Trait that defines the interface for JustKnobs supported by this library and multiple stub
/// implementations is contains.
trait JustKnobs {
    fn eval(name: &str, hash_val: Option<&str>, switch_val: Option<&str>) -> Result<bool>;

    fn get(_name: &str, switch_val: Option<&str>) -> Result<i64>;

    fn get_as<T>(name: &str, switch_val: Option<&str>) -> Result<T>
    where
        T: TryFrom<i64>,
        <T as TryFrom<i64>>::Error: std::error::Error + Send + Sync + 'static,
    {
        Ok(get(name, switch_val)?.try_into()?)
    }
}

/// For open-source for now we're using a stub implementation that always returns default.
#[cfg(not(fbcode_build))]
struct JustKnobsStub;

#[cfg(not(fbcode_build))]
impl JustKnobs for JustKnobsStub {
    fn eval(_name: &str, _hash_val: Option<&str>, _switch_val: Option<&str>) -> Result<bool> {
        Ok(false)
    }

    fn get(_name: &str, _switch_val: Option<&str>) -> Result<i64> {
        Ok(0)
    }
}

pub struct JustKnobsCombinedImpl;

impl JustKnobs for JustKnobsCombinedImpl {
    fn eval(name: &str, hash_val: Option<&str>, switch_val: Option<&str>) -> Result<bool> {
        if thread_local_in_memory::in_use() {
            ThreadLocalInMemoryJustKnobsImpl::eval(name, hash_val, switch_val)
        } else if cached_config::in_use() {
            CachedConfigJustKnobs::eval(name, hash_val, switch_val)
        } else {
            prod_implementation::eval(name, hash_val, switch_val)
        }
    }

    fn get(name: &str, switch_val: Option<&str>) -> Result<i64> {
        if thread_local_in_memory::in_use() {
            ThreadLocalInMemoryJustKnobsImpl::get(name, switch_val)
        } else if cached_config::in_use() {
            CachedConfigJustKnobs::get(name, switch_val)
        } else {
            prod_implementation::get(name, switch_val)
        }
    }
}

// There is no way to `pub use` implementation of a trait. I wish we could
// pub use <JustKnobsCombinedImpl as Justknobs>::*;
// instead of boilerplate code below.
pub fn eval(name: &str, hash_val: Option<&str>, switch_val: Option<&str>) -> Result<bool> {
    JustKnobsCombinedImpl::eval(name, hash_val, switch_val)
}

pub fn get(name: &str, switch_val: Option<&str>) -> Result<i64> {
    JustKnobsCombinedImpl::get(name, switch_val)
}

pub fn get_as<T>(name: &str, switch_val: Option<&str>) -> Result<T>
where
    T: TryFrom<i64>,
    <T as TryFrom<i64>>::Error: std::error::Error + Send + Sync + 'static,
{
    JustKnobsCombinedImpl::get_as(name, switch_val)
}
