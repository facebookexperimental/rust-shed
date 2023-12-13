/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]

//! Crate that wraps the OSS hostname and FB internal libraries to provide
//! hostname resolution

use anyhow::Result;

/// Returns hostname as reported by the system
pub fn get_hostname() -> Result<String> {
    #[cfg(not(fbcode_build))]
    {
        Ok(::real_hostname::get()?.to_string_lossy().into_owned())
    }

    #[cfg(fbcode_build)]
    {
        fbwhoami::FbWhoAmI::get()?
            .name
            .clone()
            .ok_or_else(|| ::anyhow::Error::msg("No hostname in fbwhoami"))
    }
}
