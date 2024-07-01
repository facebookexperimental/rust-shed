/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![allow(elided_lifetimes_in_paths)]

#[allow(unused_extern_crates)]
extern crate proc_macro;

mod args;
mod expand;

use proc_macro::TokenStream;
use syn::parse::Error;
use syn::parse_macro_input;
use syn::ItemFn;

use crate::args::Args;
use crate::expand::expand;
use crate::expand::Mode;

// Expand from:
//
//     #[fbinit::main]
//     fn main(fb: FacebookInit) {
//         ...
//     }
//
// to:
//
//     fn main() {
//         let fb: FacebookInit = fbinit::perform_init();
//         ...
//     }
//
// If async, also add a #[tokio::main] attribute.
//
// Accepts optional attribute argument disable_fatal_signals to disable adding
// handler to fatal signals in perform_init().
// Argument must be one of `default`, `none`, `all`, `sigterm_only`
// that represents the signal bit mask. For  example, the following disables SIGTERM:
//
//      #[fbinit::main(disable_fatal_signals = sigterm_only)
//
// - `default`: disables SIGTERM and SIGINT, and is also the default if `disable_fatal_signals`
//  is not specified
// - `none`: disabled no signals, overrides the default
// - `all`: disables ALL signals
// - `sigterm_only`: disabled SIGTERM
#[proc_macro_attribute]
pub fn main(attr: TokenStream, input: TokenStream) -> TokenStream {
    do_expand(Mode::Main, attr, input)
}

// Same thing, expand:
//
//     #[fbinit::test]
//     fn name_of_test(fb: FacebookInit) {
//         ...
//     }
//
// to:
//
//     #[test]
//     fn name_of_test() {
//         let fb: FacebookInit = fbinit::perform_init();
//         ...
//     }
//
// with either #[test] or #[tokio::test] attribute.
//
// Accepts optional attribute argument disable_fatal_signals to disable adding
// handler to fatal signals in perform_init().
// Argument must be an int literal that represents the signal bit mask. For
// example, the following disables SIGTERM:
//
//      #[fbinit::test(disable_fatal_signals = 0x8000)
#[proc_macro_attribute]
pub fn test(attr: TokenStream, input: TokenStream) -> TokenStream {
    do_expand(Mode::Test, attr, input)
}

// Similar to #[fbinit::test], but allows for nesting with other test attributes (e.g. #[rstest]) that wraps #[test].
//
//     #[fbinit::nested_test]
//     #[rstest]
//     fn name_of_test(fb: FacebookInit, some_fixture: u32) {
//         ...
//     }
//
// to:
//
//     #[rstest]
//     fn name_of_test(some_fixture: u32) {
//         let fb: FacebookInit = fbinit::perform_init();
//         ...
//     }
#[proc_macro_attribute]
pub fn nested_test(attr: TokenStream, input: TokenStream) -> TokenStream {
    do_expand(Mode::NestedTest, attr, input)
}

fn do_expand(mode: Mode, attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut args = Args::default();
    let attr_parser = syn::meta::parser(|meta| args.parse(meta));
    parse_macro_input!(attr with attr_parser);

    let input = parse_macro_input!(input as ItemFn);

    expand(mode, args, input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
