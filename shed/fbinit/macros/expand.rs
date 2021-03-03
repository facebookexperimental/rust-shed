/*
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{parse_quote, Error, ItemFn, LitInt, Result, Token};

#[derive(Copy, Clone, PartialEq)]
pub enum Mode {
    Main,
    Test,
}

mod kw {
    syn::custom_keyword!(disable_fatal_signals);
}

pub enum Arg {
    DisableFatalSignals {
        kw_token: kw::disable_fatal_signals,
        eq_token: Token![=],
        value: LitInt,
    },
}

impl Parse for Arg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::disable_fatal_signals) {
            Ok(Self::DisableFatalSignals {
                kw_token: input.parse()?,
                eq_token: input.parse()?,
                value: input.parse()?,
            })
        } else {
            Err(lookahead.error())
        }
    }
}

pub fn expand(
    mode: Mode,
    args: Punctuated<Arg, Token![,]>,
    mut function: ItemFn,
) -> Result<TokenStream> {
    let mut disable_fatal_signals = None;

    for arg in args {
        match arg {
            Arg::DisableFatalSignals { value, .. } => disable_fatal_signals = Some(value),
        }
    }

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

    let body = match (function.sig.asyncness.is_some(), mode) {
        (true, Mode::Test) => quote! {
            fbinit_tokio::tokio_test(async #block )
        },
        (true, Mode::Main) => quote! {
            fbinit_tokio::tokio_main(async #block )
        },
        (false, _) => {
            let stmts = block.stmts;
            quote! { #(#stmts)* }
        }
    };

    let perform_init = match disable_fatal_signals {
        Some(disable_fatal_signals) => {
            quote! {
                fbinit::r#impl::perform_init_with_disable_signals(#disable_fatal_signals)
            }
        }
        None => {
            quote! {
                fbinit::r#impl::perform_init()
            }
        }
    };

    function.block = parse_quote!({
        #guard
        #assignment unsafe {
            #perform_init
        };
        let destroy_guard = unsafe { fbinit::r#impl::DestroyGuard::new() };
        #body
    });

    function.sig.asyncness = None;

    if mode == Mode::Test {
        function.attrs.push(parse_quote!(#[test]));
    }

    Ok(quote!(#function))
}
