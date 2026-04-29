/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::env;

use request_context_tokio::BuilderExt as _;
use tokio::runtime;

pub(crate) fn maybe_install_request_context_hooks(builder: &mut runtime::Builder) {
    if env::var("TOKIO_RCTX_HOOKS").is_ok_and(|v| v == "1") {
        builder.install_request_context_hooks();
    }
}
