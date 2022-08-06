/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::collections::BTreeMap;

use maplit::btreemap;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct RuleBase<'a> {
    name: &'a str,
    deps: Vec<&'a str>,
    labels: Vec<&'a str>,
}

#[derive(Debug, Serialize)]
#[serde(rename = "rust_binary")]
struct RustBinary<'a> {
    #[serde(flatten)]
    base: RuleBase<'a>,
    srcs: Vec<&'a str>,
    #[serde(flatten)]
    extras: BTreeMap<&'a str, &'a str>,
}

#[derive(Serialize)]
#[serde(rename = "call:glob")]
struct Glob<T: Serialize>(T);

fn main() {
    let rule = RustBinary {
        base: RuleBase {
            name: "buck",
            deps: vec![],
            labels: vec!["foo\"'bar"],
        },
        srcs: vec!["buck.rs"],
        extras: btreemap! {
            "other" => "thing",
            "this" => "that",
        },
    };

    let s = serde_starlark::to_string(&rule).unwrap();
    println!("normal: {}", s);
    let s = serde_starlark::to_string_pretty(&rule).unwrap();
    println!("pretty: {}", s);
    let s = serde_starlark::function_call("rust_binary", &rule).unwrap();
    println!("function: {}", s);

    let s = serde_starlark::function_call("glob", &(vec!["src/**/*.rs"],)).unwrap();
    println!("glob: {}", s);

    let s = serde_starlark::to_string_pretty(&Glob((vec!["src/**/*.rs"],))).unwrap();
    println!("call:glob: {}", s);
}
