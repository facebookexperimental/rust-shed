/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::path::PathBuf;

/// Checks if the provided directory is in an EdenFS
///
/// This implements the logic recommended by
/// https://www.internalfb.com/intern/wiki/?fbid=226405021435001.
pub fn is_eden(dir: PathBuf) -> Result<bool, std::io::Error> {
    is_eden_impl(dir)
}

#[cfg(windows)]
fn is_eden_impl(mut dir: PathBuf) -> Result<bool, std::io::Error> {
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

    dir = std::fs::canonicalize(&dir)?;
    loop {
        dir.push(".eden");
        dir.push("config");
        if std::fs::metadata(&dir).map_or(false, |metadata| metadata.is_file()) {
            return Ok(true);
        }
        dir.pop();
        dir.pop();

        dir.push(".hg");
        if std::fs::metadata(&dir).map_or(false, |metadata| metadata.is_file()) {
            return Ok(false);
        }
        dir.pop();

        dir.push(".git");
        if std::fs::metadata(&dir).map_or(false, |metadata| metadata.is_file()) {
            return Ok(false);
        }
        dir.pop();

        if is_mount_point(dir.clone())? {
            return Ok(false);
        }

        if !dir.pop() {
            return Ok(false);
        }
    }
}

#[cfg(not(windows))]
fn is_eden_impl(mut dir: PathBuf) -> Result<bool, std::io::Error> {
    dir.push(".eden");
    dir.push("root");
    Ok(std::fs::read_link(&dir).is_ok())
}
