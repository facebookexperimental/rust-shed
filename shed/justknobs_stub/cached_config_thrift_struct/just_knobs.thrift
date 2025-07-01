/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

include "thrift/annotation/rust.thrift"

@rust.Exhaustive
struct JustKnobs {
  1: JustKnobInts ints;
  2: JustKnobBools bools;
}

@rust.Type{name = "HashMap"}
typedef map<string, bool> JustKnobBools
@rust.Type{name = "HashMap"}
typedef map<string, i64> JustKnobInts
