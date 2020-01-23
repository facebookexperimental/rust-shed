/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use slog::{debug, error, info, o};

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
