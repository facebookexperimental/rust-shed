/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

#![deny(warnings)]
#![feature(never_type)]

#[cfg(fbcode_build)]
#[path = "facebook/native.rs"]
mod facebook;

#[cfg(not(fbcode_build))]
mod oss;

#[cfg(fbcode_build)]
pub use services_common::*;

#[cfg(fbcode_build)]
pub use crate::facebook::*;
#[cfg(not(fbcode_build))]
pub use crate::oss::*;

#[cfg(all(test, not(fbcode_build)))]
mod tests {
    #[test]
    fn proc_stats_is_a_no_op() {
        super::export_proc_stats_to_fb303(true);
        super::export_proc_stats_to_fb303(false);
    }
}
