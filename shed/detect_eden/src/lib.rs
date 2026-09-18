/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

#[cfg(windows)]
use std::fs::Metadata;
use std::io;
#[cfg(windows)]
use std::path::Path;
use std::path::PathBuf;

/// Checks if the provided directory is in an EdenFS
///
/// This implements the logic recommended by
/// https://www.internalfb.com/intern/wiki/?fbid=226405021435001.
pub fn is_eden(dir: PathBuf) -> Result<bool, io::Error> {
    find_eden_root(dir).map(|eden_root| eden_root.is_some())
}

/// Find the EdenFS root for the provided directory.
///
/// Marker lookup and resolution failures are treated as absent markers. On
/// Windows, canonicalizing the input or inspecting mount points can still fail.
pub fn find_eden_root(dir: PathBuf) -> Result<Option<PathBuf>, io::Error> {
    find_eden_root_impl(dir, false)
}

/// Find the EdenFS root, preserving marker lookup and resolution errors.
///
/// Unlike [`find_eden_root`], returns an error when a filesystem failure prevents
/// detecting an Eden marker or resolving the checkout root.
pub fn find_eden_root_strict(dir: PathBuf) -> Result<Option<PathBuf>, io::Error> {
    find_eden_root_impl(dir, true)
}

#[cfg(windows)]
fn find_eden_root_impl(
    dir: PathBuf,
    preserve_marker_errors: bool,
) -> Result<Option<PathBuf>, io::Error> {
    find_eden_root_with_metadata(dir, preserve_marker_errors, |path| std::fs::metadata(path))
}

#[cfg(windows)]
fn find_eden_root_with_metadata(
    mut dir: PathBuf,
    preserve_marker_errors: bool,
    mut metadata: impl FnMut(&Path) -> io::Result<Metadata>,
) -> Result<Option<PathBuf>, io::Error> {
    /// Implemented as described in
    /// https://docs.microsoft.com/en-us/windows/win32/fileio/determining-whether-a-directory-is-a-volume-mount-point
    fn is_mount_point(mut dir: PathBuf) -> Result<bool, io::Error> {
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
                return Err(io::Error::new(
                    io::ErrorKind::Other,
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
        let is_confirmed_repo = match metadata(&dir) {
            Ok(metadata) => metadata.is_file(),
            Err(error)
                if !preserve_marker_errors
                    || matches!(
                        error.kind(),
                        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory
                    ) =>
            {
                false
            }
            Err(error) => return Err(error),
        };
        dir.pop();
        dir.pop();
        if is_confirmed_repo {
            return Ok(Some(dir));
        }

        dir.push(".hg");
        if metadata(&dir).is_ok_and(|metadata| metadata.is_file()) {
            return Ok(None);
        }
        dir.pop();

        dir.push(".git");
        if metadata(&dir).is_ok_and(|metadata| metadata.is_file()) {
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
fn find_eden_root_impl(
    mut dir: PathBuf,
    preserve_marker_errors: bool,
) -> Result<Option<PathBuf>, io::Error> {
    dir.push(".eden");
    dir.push("root");
    let result = match std::fs::read_link(&dir) {
        Ok(_) => dunce::canonicalize(&dir).map(Some),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound
                    | io::ErrorKind::NotADirectory
                    | io::ErrorKind::InvalidInput
            ) =>
        {
            Ok(None)
        }
        Err(error) => Err(error),
    };
    if preserve_marker_errors {
        result
    } else {
        Ok(result.ok().flatten())
    }
}

#[cfg(all(test, unix))]
mod tests {
    use std::fs;
    use std::os::unix::fs::symlink;

    use super::*;

    #[test]
    fn resolves_relative_root_marker_from_marker_location() -> io::Result<()> {
        let temp = tempfile::tempdir()?;
        let checkout = temp.path().join("checkout");
        let directory = checkout.join("directory");
        let eden_dir = directory.join(".eden");
        fs::create_dir_all(&eden_dir)?;
        symlink("../..", eden_dir.join("root"))?;

        let expected = Some(dunce::canonicalize(checkout)?);
        assert_eq!(find_eden_root(directory.clone())?, expected);
        assert_eq!(find_eden_root_strict(directory.clone())?, expected);
        assert!(is_eden(directory)?);
        Ok(())
    }

    #[test]
    fn returns_none_when_root_marker_is_absent() -> io::Result<()> {
        let temp = tempfile::tempdir()?;

        assert_eq!(find_eden_root(temp.path().to_path_buf())?, None);
        assert_eq!(find_eden_root_strict(temp.path().to_path_buf())?, None);
        assert!(!is_eden(temp.path().to_path_buf())?);
        Ok(())
    }

    #[test]
    fn returns_none_when_root_marker_is_not_a_symlink() -> io::Result<()> {
        let temp = tempfile::tempdir()?;
        let eden_dir = temp.path().join(".eden");
        fs::create_dir(&eden_dir)?;
        fs::write(eden_dir.join("root"), "not a symlink")?;

        assert_eq!(find_eden_root(temp.path().to_path_buf())?, None);
        assert_eq!(find_eden_root_strict(temp.path().to_path_buf())?, None);
        Ok(())
    }

    #[test]
    fn returns_error_when_root_marker_cannot_be_resolved() -> io::Result<()> {
        let temp = tempfile::tempdir()?;
        let eden_dir = temp.path().join(".eden");
        fs::create_dir(&eden_dir)?;
        symlink("missing", eden_dir.join("root"))?;

        assert_eq!(find_eden_root(temp.path().to_path_buf())?, None);
        assert!(!is_eden(temp.path().to_path_buf())?);
        let error = find_eden_root_strict(temp.path().to_path_buf())
            .expect_err("a dangling root marker must return its resolution error");
        assert_eq!(error.kind(), io::ErrorKind::NotFound);
        Ok(())
    }

    #[test]
    fn only_strict_detection_returns_marker_lookup_errors() -> io::Result<()> {
        let temp = tempfile::tempdir()?;
        symlink(".eden", temp.path().join(".eden"))?;

        assert_eq!(find_eden_root(temp.path().to_path_buf())?, None);
        assert!(!is_eden(temp.path().to_path_buf())?);
        find_eden_root_strict(temp.path().to_path_buf())
            .expect_err("a symlink loop must return its lookup error");
        Ok(())
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use std::fs;

    use super::*;

    #[test]
    fn ignores_unreadable_scm_markers() -> io::Result<()> {
        for (marker, eden_root) in [
            (".hg", false),
            (".git", false),
            (".hg", true),
            (".git", true),
        ] {
            let temp = tempfile::tempdir()?;
            let ancestor = temp.path().join("ancestor");
            let project = ancestor.join("project");
            fs::create_dir_all(&project)?;
            fs::write(temp.path().join(".git"), "boundary")?;
            let expected = if eden_root {
                fs::create_dir(temp.path().join(".eden"))?;
                fs::write(temp.path().join(".eden/config"), "")?;
                Some(dunce::canonicalize(temp.path())?)
            } else {
                None
            };

            let marker_path = ancestor.join(marker);
            fs::create_dir(&marker_path)?;
            let marker_path = dunce::canonicalize(marker_path)?;
            let mut queried_marker = false;
            let actual = find_eden_root_with_metadata(project, true, |path| {
                if path == marker_path {
                    queried_marker = true;
                    Err(io::ErrorKind::PermissionDenied.into())
                } else {
                    fs::metadata(path)
                }
            })?;
            assert!(queried_marker, "the {marker} lookup must return an error");
            assert_eq!(
                actual, expected,
                "an unreadable {marker} must not prevent Eden detection"
            );
        }
        Ok(())
    }

    #[test]
    fn preserves_config_permission_errors() -> io::Result<()> {
        let temp = tempfile::tempdir()?;
        let eden = temp.path().join(".eden");
        let config = eden.join("config");
        fs::create_dir(&eden)?;
        fs::write(&config, "")?;
        fs::write(temp.path().join(".git"), "boundary")?;
        let config = dunce::canonicalize(config)?;

        let error = find_eden_root_with_metadata(temp.path().to_path_buf(), true, |path| {
            if path == config {
                Err(io::ErrorKind::PermissionDenied.into())
            } else {
                fs::metadata(path)
            }
        })
        .expect_err("an inaccessible Eden config must return an error");
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        Ok(())
    }

    #[test]
    fn preserves_invalid_input_from_metadata() -> io::Result<()> {
        let temp = tempfile::tempdir()?;
        let error = find_eden_root_with_metadata(temp.path().to_path_buf(), true, |_| {
            fs::metadata(Path::new("invalid\0path"))
        })
        .expect_err("invalid metadata requests must return an error");
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        Ok(())
    }

    #[test]
    fn legacy_detection_continues_past_config_errors() -> io::Result<()> {
        let temp = tempfile::tempdir()?;
        fs::create_dir(temp.path().join(".eden"))?;
        fs::write(temp.path().join(".eden/config"), "")?;
        let project = temp.path().join("project");
        fs::create_dir_all(project.join(".eden"))?;
        fs::write(project.join(".eden/config"), "")?;
        let config = dunce::canonicalize(project.join(".eden/config"))?;

        for kind in [
            io::ErrorKind::PermissionDenied,
            io::ErrorKind::InvalidInput,
            io::ErrorKind::NotConnected,
        ] {
            let metadata = |path: &Path| {
                if path == config {
                    Err(kind.into())
                } else {
                    fs::metadata(path)
                }
            };
            assert_eq!(
                find_eden_root_with_metadata(project.clone(), false, metadata)?,
                Some(dunce::canonicalize(temp.path())?)
            );
            let error = find_eden_root_with_metadata(project.clone(), true, metadata)
                .expect_err("strict detection must preserve config lookup errors");
            assert_eq!(error.kind(), kind);
        }
        Ok(())
    }

    #[test]
    fn legacy_detection_preserves_input_canonicalization_errors() -> io::Result<()> {
        let temp = tempfile::tempdir()?;
        let missing = temp.path().join("missing");

        assert_eq!(
            find_eden_root(missing.clone()).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            find_eden_root_strict(missing.clone()).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        assert_eq!(
            is_eden(missing).unwrap_err().kind(),
            io::ErrorKind::NotFound
        );
        Ok(())
    }
}
