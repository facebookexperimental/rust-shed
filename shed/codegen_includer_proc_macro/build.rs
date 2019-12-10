/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let lib_rs = Path::new(&env::var("OUT_DIR").unwrap()).join("lib.rs");
    fs::copy("tests/fixtures/lib.rs", lib_rs).expect("Failed copying fixtures");
}
