/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::thread;

use panichandler::Fate;

fn main() {
    println!("I'm on an adventure!");

    panichandler::set_panichandler(Fate::Exit(99));

    let t = thread::spawn(|| panic!("I paniced! {} {}", "Everything's awful!", 1234));
    let _ = t.join();

    println!("I shouldn't have returned");
}
