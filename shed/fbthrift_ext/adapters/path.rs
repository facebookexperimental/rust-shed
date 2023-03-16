/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Adapters that interpret thrift types as various path types.

use camino::Utf8PathBuf;
use fbthrift::adapter::NewTypeAdapter;

/// Adapts thrift strings as [`Utf8PathBuf`]s.
///
/// # Examples
///
/// ```thrift
/// include "thrift/annotation/rust.thrift";
///
/// @rust.Adapter{name = "::fbthrift_adapters::Utf8PathAdapter"}
/// typedef string path;
///
/// struct CreateWorkflowRequest {
///   1: path my_path;
/// }
/// ```
///
/// [`Utf8PathBuf`]: camino::Utf8PathBuf
pub struct Utf8PathAdapter;

impl NewTypeAdapter for Utf8PathAdapter {
    type StandardType = String;
    type AdaptedType = Utf8PathBuf;
}
