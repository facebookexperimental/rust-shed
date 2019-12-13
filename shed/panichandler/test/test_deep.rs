/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

use std::sync::atomic::{AtomicUsize, Ordering};

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
