/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::path::PathBuf;

/// Checks if the provided directory is in an EdenFS
///
/// This implements the logic recommended by
/// https://www.internalfb.com/intern/wiki/?fbid=226405021435001.
pub fn is_eden(dir: PathBuf) -> Result<bool, std::io::Error> {
    find_eden_root(dir).map(|eden_root| eden_root.is_some())
}

/// Find the EdenFS root for the provided directory.
pub fn find_eden_root(dir: PathBuf) -> Result<Option<PathBuf>, std::io::Error> {
    find_eden_root_impl(dir)
}

#[cfg(windows)]
fn find_eden_root_impl(mut dir: PathBuf) -> Result<Option<PathBuf>, std::io::Error> {
    /// Implemented as described in
    /// https://docs.microsoft.com/en-us/windows/win32/fileio/determining-whether-a-directory-is-a-volume-mount-point
    fn is_mount_point(mut dir: PathBuf) -> Result<bool, std::io::Error> {
        use std::mem::MaybeUninit;
        use std::os::windows::ffi::OsStrExt;
        use std::os::windows::fs::MetadataExt;

        // Append a `\` to the end of the directory path
        if !dir.ends_with("") {
            dir.push("");
        }
        let mut encoded = dir.as_os_str().encode_wide().collect::<Vec<u16>>();
        encoded.push(0);

        unsafe {
            let metadata = std::fs::metadata(&dir)?;

            if metadata.file_attributes() & winapi::um::winnt::FILE_ATTRIBUTE_REPARSE_POINT == 0 {
                return Ok(false);
            }

            let mut data: MaybeUninit<winapi::um::minwinbase::WIN32_FIND_DATAW> =
                MaybeUninit::uninit();
            let data_ptr = data.as_mut_ptr();
            let handle = winapi::um::fileapi::FindFirstFileW(encoded.as_ptr(), data_ptr);
            if handle == winapi::um::handleapi::INVALID_HANDLE_VALUE {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "invalid handle value".to_string(),
                ));
            }
            winapi::um::fileapi::FindClose(handle);

            let data = data.assume_init();
            Ok(data.dwReserved0 == winapi::um::winnt::IO_REPARSE_TAG_MOUNT_POINT)
        }
    }

    dir = dunce::canonicalize(&dir)?;
    loop {
        dir.push(".eden");
        dir.push("config");
        let is_confirmed_repo =
            std::fs::metadata(&dir).map_or(false, |metadata| metadata.is_file());
        dir.pop();
        dir.pop();
        if is_confirmed_repo {
            return Ok(Some(dir));
        }

        dir.push(".hg");
        if std::fs::metadata(&dir).map_or(false, |metadata| metadata.is_file()) {
            return Ok(None);
        }
        dir.pop();

        dir.push(".git");
        if std::fs::metadata(&dir).map_or(false, |metadata| metadata.is_file()) {
            return Ok(None);
        }
        dir.pop();

        if is_mount_point(dir.clone())? {
            return Ok(None);
        }

        if !dir.pop() {
            return Ok(None);
        }
    }
}

#[cfg(not(windows))]
fn find_eden_root_impl(mut dir: PathBuf) -> Result<Option<PathBuf>, std::io::Error> {
    dir.push(".eden");
    dir.push("root");
    // The `canonicalize` is not strictly necessary on the non-windows path, but we do it on Windows
    // and so this is more consistent.
    Ok(std::fs::read_link(&dir).and_then(dunce::canonicalize).ok())
}
