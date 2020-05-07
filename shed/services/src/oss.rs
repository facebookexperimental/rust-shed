/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use fbinit::FacebookInit;
use std::thread::sleep;
use std::time::Duration;

pub use services_common::*;

pub fn export_proc_stats_to_fb303(_: bool) {}

pub fn run_service_framework<T: Into<Vec<u8>>>(
    _: FacebookInit,
    _: T,
    _: i32,
    _: i32,
    _: Box<dyn Fb303Service>,
) -> Result<!> {
    loop {
        sleep(Duration::from_secs(3600))
    }
}
