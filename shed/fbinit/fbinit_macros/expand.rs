/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_quote;
use syn::punctuated::Punctuated;
use syn::Error;
use syn::ItemFn;
use syn::Result;
use syn::Token;

#[derive(Copy, Clone, PartialEq)]
pub enum Mode {
    Main,
    Test,
}

mod kw {
    syn::custom_keyword!(disable_fatal_signals);
    syn::custom_keyword!(none);
    syn::custom_keyword!(sigterm_only);
    syn::custom_keyword!(all);
    syn::custom_keyword!(worker_threads);
}

pub enum DisableFatalSignals {
    Default(Token![default]),
    None(kw::none),
    SigtermOnly(kw::sigterm_only),
    All(kw::all),
}

pub enum Arg {
    DisableFatalSignals {
        kw_token: kw::disable_fatal_signals,
        eq_token: Token![=],
        value: DisableFatalSignals,
    },
    TokioWorkers {
        kw_token: kw::worker_threads,
        eq_token: Token![=],
        workers: syn::LitInt,
    },
}

impl Parse for Arg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::disable_fatal_signals) {
            let kw_token = input.parse()?;
            let eq_token = input.parse()?;

            let lookahead = input.lookahead1();
            let value = if lookahead.peek(kw::none) {
                DisableFatalSignals::None(input.parse()?)
            } else if lookahead.peek(Token![default]) {
                DisableFatalSignals::Default(input.parse()?)
            } else if lookahead.peek(kw::all) {
                DisableFatalSignals::All(input.parse()?)
            } else if lookahead.peek(kw::sigterm_only) {
                DisableFatalSignals::SigtermOnly(input.parse()?)
            } else {
                return Err(lookahead.error());
            };

            Ok(Self::DisableFatalSignals {
                kw_token,
                eq_token,
                value,
            })
        } else if lookahead.peek(kw::worker_threads) {
            let kw_token = input.parse()?;
            let eq_token = input.parse()?;
            let workers = input.parse()?;
            Ok(Self::TokioWorkers {
                kw_token,
                eq_token,
                workers,
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
    let mut disable_fatal_signals =
        DisableFatalSignals::Default(syn::parse2(quote! { default }).expect("This always parses"));

    let mut tokio_workers: usize = 0;
    for arg in args {
        match arg {
            Arg::DisableFatalSignals { value, .. } => disable_fatal_signals = value,
            Arg::TokioWorkers { workers, .. } => tokio_workers = workers.base10_parse()?,
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
            fbinit_tokio::tokio_main(#tokio_workers, async #block )
        },
        (false, _) => {
            let stmts = block.stmts;
            quote! { #(#stmts)* }
        }
    };

    let perform_init = match disable_fatal_signals {
        DisableFatalSignals::Default(_) => {
            // 8002 is 1 << 15 (SIGTERM) | 1 << 2 (SIGINT)
            quote! {
                fbinit::internal::perform_init_with_disable_signals(0x8002)
            }
        }
        DisableFatalSignals::All(_) => {
            // ffff is a mask of all 1's
            quote! {
                fbinit::internal::perform_init_with_disable_signals(0xffff)
            }
        }
        DisableFatalSignals::SigtermOnly(_) => {
            // 8000 is 1 << 15 (SIGTERM)
            quote! {
                fbinit::internal::perform_init_with_disable_signals(0x8000)
            }
        }
        DisableFatalSignals::None(_) => {
            quote! {
                fbinit::perform_init()
            }
        }
    };

    function.block = parse_quote!({
        #guard
        #assignment unsafe {
            #perform_init
        };
        let destroy_guard = unsafe { fbinit::internal::DestroyGuard::new() };
        #body
    });

    function.sig.asyncness = None;

    if mode == Mode::Test {
        function.attrs.push(parse_quote!(#[test]));
    }

    Ok(quote!(#function))
}
