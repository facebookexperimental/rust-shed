/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

struct JustKnobs {
  1: JustKnobInts ints;
  2: JustKnobBools bools;
} (rust.exhaustive)

typedef map<string, bool> (rust.type = "HashMap") JustKnobBools
typedef map<string, i64> (rust.type = "HashMap") JustKnobInts
