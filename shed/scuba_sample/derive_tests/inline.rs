/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use ::scuba_sample::ScubaSample;
use ::scuba_sample::StructuredSample;

/// Make sure that we can declare a sample inline within a function
#[test]
fn test_basic() {
    #[derive(StructuredSample)]
    struct Basic {
        field: i32,
    }

    let _sample: ScubaSample = Basic { field: 5 }.into();
}
