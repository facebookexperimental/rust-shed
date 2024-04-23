/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::Ok;
use procfs::process::Process;

/// A memory bound that serves as the upper bound for the RSS bytes of a process that
/// should always be honored when scheduling new workload.
#[derive(Debug)]
pub(crate) struct MemoryBound {
    bound: Option<u64>,
}

impl MemoryBound {
    pub(crate) fn new(bound: Option<u64>) -> Self {
        Self { bound }
    }

    /// Returns true if the RSS bytes of the process would still remain within
    /// the `bound` after scheduling the future of `weight` bytes.
    pub(crate) fn within_bound(&self, weight: usize) -> bool {
        self.bound
            .map_or(Ok(true), |bound| {
                let stats = Process::myself()?.stat()?;
                let page_size = procfs::page_size();
                let rss_bytes = stats.rss * page_size;
                let next_rss_bytes = rss_bytes.saturating_add(weight as u64);
                Ok(next_rss_bytes < bound)
            })
            .unwrap_or(true)
    }
}