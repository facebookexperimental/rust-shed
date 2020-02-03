/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This software may be used and distributed according to the terms of the
 * GNU General Public License found in the LICENSE file in the root
 * directory of this source tree.
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
        Mode::Test => None,
    };

    let assignment = function.sig.inputs.first().map(|arg| quote!(let #arg =));
    function.sig.inputs = Punctuated::new();

    let block = function.block;
    function.block = parse_quote!({
        #guard
        #assignment unsafe {
            fbinit::r#impl::perform_init()
        };
        #block
    });

    if function.sig.asyncness.is_some() {
        let tokio_attribute = match mode {
            Mode::Main => "main",
            Mode::Test => "test",
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
