/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parse_quote, Error, Ident, ItemFn, Result};

#[derive(Copy, Clone, PartialEq)]
pub enum Mode {
    Main,
    Test,
    CompatTest,
}

pub fn expand(mode: Mode, mut function: ItemFn) -> Result<TokenStream> {
    if function.sig.inputs.len() > 1 {
        return Err(Error::new_spanned(
            function.sig,
            "expected one argument of type fbinit::FacebookInit",
        ));
    }

    if mode == Mode::Main && function.sig.ident != "main" {
        return Err(Error::new_spanned(
            function.sig,
            "#[fbinit::main] must be used on the main function",
        ));
    }

    let guard = match mode {
        Mode::Main => Some(quote! {
            if module_path!().contains("::") {
                panic!("fbinit must be performed in the crate root on the main function");
            }
        }),
        Mode::Test | Mode::CompatTest => None,
    };

    let assignment = function.sig.inputs.first().map(|arg| quote!(let #arg =));
    function.sig.inputs = Punctuated::new();

    let block = function.block;
    function.block = parse_quote!({
        #guard
        #assignment unsafe {
            fbinit::r#impl::perform_init()
        };
        let _destroy_guard = unsafe { fbinit::r#impl::DestroyGuard::new() };
        #block
    });

    if mode == Mode::CompatTest {
        let block = function.block;
        function.block = parse_quote!({
            tokio_compat::runtime::current_thread::Runtime::new().unwrap().block_on_std(async {
                #block
            })
        });
        if function.sig.asyncness.is_none() {
            return Err(Error::new_spanned(
                function.sig,
                "#[fbinit::compat_test] should be used only on async functions",
            ));
        }
        function.sig.asyncness = None;
        function.attrs.push(parse_quote!(#[test]));

        Ok(quote!(#function))
    } else {
        if function.sig.asyncness.is_some() {
            let tokio_attribute = match mode {
                Mode::Main => "main",
                Mode::Test => "test",
                Mode::CompatTest => unreachable!(),
            };
            let span = function.sig.span();
            let ident = Ident::new(tokio_attribute, span);
            let attr = quote_spanned! {span=>
                #[fbinit::r#impl::tokio::#ident]
            };
            function.attrs.push(parse_quote!(#attr));
        } else if mode == Mode::Test {
            function.attrs.push(parse_quote!(#[test]));
        }

        Ok(quote!(#function))
    }
}
