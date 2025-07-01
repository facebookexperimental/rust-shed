/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

static COUNT: AtomicUsize = AtomicUsize::new(0);

fn main() {
    println!("I'm on an adventure!");

    panichandler::set_panichandler(panichandler::Fate::Continue);

    #[allow(unconditional_recursion)]
    fn deep(d: u32) {
        if d == 0 {
            panic!("I paniced! {} {}", "Everything's awful!", 1234);
        } else {
            deep(d - 1);
            let _ = COUNT.fetch_add(1, Ordering::SeqCst);
        }
    }

    deep(2000);
}
