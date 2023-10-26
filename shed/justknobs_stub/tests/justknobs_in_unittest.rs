/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use anyhow::Result;
use justknobs::test_helpers::*;
use maplit::hashmap;

#[test]
fn test_jk_override() -> Result<()> {
    let res = with_just_knobs(
        JustKnobsInMemory::new(hashmap! {
            "my/config:knob1".to_string() => KnobVal::Bool(true),
            "my/config:knob2".to_string() => KnobVal::Int(2),
        }),
        || {
            (
                justknobs::eval("my/config:knob1", None, None).unwrap(),
                justknobs::get("my/config:knob2", None).unwrap(),
            )
        },
    );
    assert_eq!(res, (true, 2));
    Ok(())
}
