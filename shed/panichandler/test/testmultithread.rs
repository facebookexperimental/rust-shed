/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
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
