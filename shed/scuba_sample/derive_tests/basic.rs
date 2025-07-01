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
use ::scuba_sample::ScubaValue;
use ::scuba_sample::StructuredSample;
use ::scuba_sample::TryFromSample;

#[derive(StructuredSample, TryFromSample, PartialEq, Debug, Clone)]
struct Basic {
    foo: i32,
    bar: String,
    fizz: bool,
}

#[test]
fn test_basic() {
    let basic = Basic {
        foo: 5,
        bar: "fizzbuzz".into(),
        fizz: false,
    };
    let sample: ScubaSample = basic.clone().into();

    assert_eq!(sample.get("foo"), Some(ScubaValue::Int(5)).as_ref());
    assert_eq!(
        sample.get("bar"),
        Some(ScubaValue::Normal("fizzbuzz".into())).as_ref()
    );
    assert_eq!(
        sample.get("fizz"),
        Some(ScubaValue::Normal("false".into())).as_ref()
    );

    let converted_basic: Basic = sample.try_into().unwrap();
    assert_eq!(converted_basic, basic);
}
