/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use ::scuba_sample::ScubaSample;
use ::scuba_sample::ScubaValue;
use ::scuba_sample::StructuredSample;

#[derive(StructuredSample)]
struct Customized {
    foo: i32,
    #[scuba(name = "bar2")]
    bar: String,
    fizz: bool,
}

#[test]
fn test_customized() {
    let sample: ScubaSample = Customized {
        foo: 5,
        bar: "fizzbuzz".into(),
        fizz: false,
    }
    .into();

    assert_eq!(sample.get("foo"), Some(ScubaValue::Int(5)).as_ref());
    assert_eq!(
        sample.get("bar2"),
        Some(ScubaValue::Normal("fizzbuzz".into())).as_ref()
    );
    assert_eq!(
        sample.get("fizz"),
        Some(ScubaValue::Normal("false".into())).as_ref()
    );
}
