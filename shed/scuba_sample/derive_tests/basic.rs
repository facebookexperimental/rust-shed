/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use ::scuba_sample::{ScubaSample, ScubaValue, StructuredSample};

#[derive(StructuredSample)]
struct Basic {
    foo: i32,
    bar: String,
    fizz: bool,
}

#[test]
fn test_basic() {
    let sample: ScubaSample = Basic {
        foo: 5,
        bar: "fizzbuzz".into(),
        fizz: false,
    }
    .into();

    assert_eq!(sample.get("foo"), Some(ScubaValue::Int(5)).as_ref());
    assert_eq!(
        sample.get("bar"),
        Some(ScubaValue::Normal("fizzbuzz".into())).as_ref()
    );
    assert_eq!(
        sample.get("fizz"),
        Some(ScubaValue::Normal("false".into())).as_ref()
    );
}
