/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
 */

::codegen_includer_proc_macro::include!();

#[test]
fn test_includer() {
    assert_eq!("Hello world!", helloWorld());
}
