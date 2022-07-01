/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use slog::debug;
use slog::error;
use slog::info;
use slog::o;

pub fn main() {
    let log = slog::Logger::root(slog_glog_fmt::default_drain(), o!());
    info!(log, "Logger started");

    {
        let sublog = log.new(o!("sublog" => 1));
        debug!(sublog, "Sublogger logging");
        error!(sublog, "Example error");
    }

    info!(log, "Logger finished");
}
