/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::thread::sleep;
use std::time::Duration;

use fbinit::FacebookInit;
pub use services_common::*;

pub fn export_proc_stats_to_fb303(_: bool) {}

pub fn run_service_framework<T: Into<Vec<u8>>>(
    _: FacebookInit,
    _: T,
    _: i32,
    _: i32,
    _: Box<dyn Fb303Service>,
) -> Result<!, ServicesError> {
    loop {
        sleep(Duration::from_secs(3600))
    }
}
