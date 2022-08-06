/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

//! Enables exposing counters for number of slog records per level

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]

use slog::Drain;
use slog::Level;
use slog::OwnedKVList;
use slog::Record;
use stats::prelude::*;

define_stats! {
    prefix = "logging";
    critical: counter(),
    error: counter(),
    warning: counter(),
    info: counter(),
    debug: counter(),
    trace: counter(),
}

/// Drain that counts number of slog records per level using `stats` crate counters
pub struct StatsDrain<D> {
    drain: D,
}

impl<D: Drain> StatsDrain<D> {
    /// Create a `StatsDrain` that will pass all records and values to the given drain unchanged
    /// and return the result of logging using that drain unchanged
    pub fn new(drain: D) -> Self {
        StatsDrain { drain }
    }
}

impl<D: Drain> Drain for StatsDrain<D> {
    type Ok = D::Ok;
    type Err = D::Err;

    fn log(&self, record: &Record<'_>, values: &OwnedKVList) -> Result<Self::Ok, Self::Err> {
        match record.level() {
            Level::Critical => STATS::critical.increment_value(1),
            Level::Error => STATS::error.increment_value(1),
            Level::Warning => STATS::warning.increment_value(1),
            Level::Info => STATS::info.increment_value(1),
            Level::Debug => STATS::debug.increment_value(1),
            Level::Trace => STATS::trace.increment_value(1),
        }

        self.drain.log(record, values)
    }
}

#[cfg(test)]
mod tests {
    use slog::info;
    use slog::o;
    use slog::Discard;
    use slog::Logger;

    use super::*;

    #[test]
    fn test() {
        let log = Logger::root(StatsDrain::new(Discard), o![]);
        info!(log, "test log");
    }
}
