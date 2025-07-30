/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

#[cfg(target_os = "linux")]
use procfs::process::Process;

/// A memory bound that serves as the upper bound for the RSS bytes of a process that
/// should always be honored when scheduling new workload.
#[derive(Debug)]
#[cfg_attr(not(target_os = "linux"), allow(unused))]
pub struct MemoryBound {
    bound: Option<u64>,
}

impl MemoryBound {
    /// Creates a new memory bound.
    pub fn new(bound: Option<u64>) -> Self {
        Self { bound }
    }

    /// Returns true if the RSS bytes of the process would still remain within
    /// the `bound` after scheduling the future of `weight` bytes.
    #[cfg(target_os = "linux")]
    pub fn within_bound(&self, weight: usize) -> bool {
        if let Some(bound) = self.bound
            && let Ok(proc) = Process::myself()
            && let Ok(stats) = proc.stat()
        {
            let page_size = procfs::page_size();
            let rss_bytes = stats.rss * page_size;
            let next_rss_bytes = rss_bytes.saturating_add(weight as u64);
            next_rss_bytes < bound
        } else {
            true
        }
    }

    /// Returns true on non-linux platforms as memory bounds are not supported.
    #[cfg(not(target_os = "linux"))]
    pub fn within_bound(&self, weight: usize) -> bool {
        // Memory bound not supported on this platform.
        true
    }
}
