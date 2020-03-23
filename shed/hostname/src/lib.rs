/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

#![deny(warnings, missing_docs, clippy::all, intra_doc_link_resolution_failure)]

//! Crate that wraps the OSS hostname and FB internal libraries to provide
//! hostname resolution

use anyhow::Result;

/// Returns hostname as reported by the system
pub fn get_hostname() -> Result<String> {
    #[cfg(not(fbcode_build))]
    {
        Ok(::hostname::get()?.to_string_lossy().into_owned())
    }

    #[cfg(fbcode_build)]
    {
        fbwhoami::FbWhoAmI::get()?
            .name
            .clone()
            .ok_or_else(|| ::anyhow::Error::msg("No hostname in fbwhoami"))
    }
}
