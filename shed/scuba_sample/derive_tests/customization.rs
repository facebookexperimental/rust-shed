/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::collections::HashMap;

use ::scuba_sample::ScubaSample;
use ::scuba_sample::ScubaValue;
use ::scuba_sample::StructuredSample;
use ::scuba_sample::TryFromSample;

struct SomeUnserializeableType;

#[derive(StructuredSample)]
struct Customized<'a> {
    foo: i32,
    #[scuba(name = "bar2")]
    bar: String,
    baz: &'a str,
    fizz: bool,
    #[allow(dead_code)]
    #[scuba(skip)]
    skipped: SomeUnserializeableType,
}

#[test]
fn test_customized() {
    let sample: ScubaSample = Customized {
        foo: 5,
        bar: "fizzbuzz".into(),
        baz: "baz",
        fizz: false,
        skipped: SomeUnserializeableType,
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

    assert_eq!(sample.get("skipped"), None);
}

fn my_custom_parser(data: String) -> Result<HashMap<String, String>, serde_json::Error> {
    serde_json::from_str(&data)
}

#[derive(TryFromSample, PartialEq, Debug, Clone)]
struct CustomizedParsing {
    foo: i32,
    #[scuba(name = "bar2")]
    bar: String,
    fizz: bool,
    #[scuba(name = "map2", custom_parser = "my_custom_parser")]
    map: HashMap<String, String>,
}

#[test]
fn test_customized_parser() {
    let mut sample = ScubaSample::new();
    sample.add("foo", 5);
    sample.add("bar2", "fizzbuzz");
    sample.add("fizz", false);
    sample.add("map2", r#"{"a": "b"}"#);
    let expected = CustomizedParsing {
        foo: 5,
        bar: "fizzbuzz".into(),
        fizz: false,
        map: vec![("a".to_owned(), "b".to_owned())].into_iter().collect(),
    };

    let actual: CustomizedParsing = sample.try_into().unwrap();

    assert_eq!(actual, expected);
}

#[derive(StructuredSample)]
struct FlattenedSample {
    foo: i64,
    fizz: bool,
}

struct DynamicColumns(HashMap<String, i64>);

impl From<DynamicColumns> for ScubaSample {
    fn from(value: DynamicColumns) -> Self {
        let mut sample = ScubaSample::new();
        for (key, value) in value.0 {
            sample.add(format!("dynamic_{key}"), value);
        }
        sample
    }
}

#[derive(StructuredSample)]
struct CustomizedWithFlatten {
    #[scuba(flatten)]
    flattened: FlattenedSample,
    bar: String,
    #[scuba(flatten)]
    dynamic_fields: DynamicColumns,
}

#[test]
fn test_flattened_sample() {
    let sample: ScubaSample = CustomizedWithFlatten {
        flattened: FlattenedSample { foo: 5, fizz: true },
        bar: "bar".into(),
        dynamic_fields: DynamicColumns(HashMap::from_iter([("x".into(), 10), ("y".into(), 20)])),
    }
    .into();

    assert_eq!(sample.get("foo"), Some(ScubaValue::Int(5)).as_ref());
    assert_eq!(
        sample.get("fizz"),
        Some(ScubaValue::Normal("true".into())).as_ref()
    );
    assert_eq!(
        sample.get("bar"),
        Some(ScubaValue::Normal("bar".into())).as_ref()
    );
    assert_eq!(sample.get("dynamic_x"), Some(ScubaValue::Int(10)).as_ref());
    assert_eq!(sample.get("dynamic_y"), Some(ScubaValue::Int(20)).as_ref());
}
