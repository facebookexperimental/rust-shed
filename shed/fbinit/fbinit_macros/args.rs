/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse::Result;
use syn::LitInt;
use syn::Token;

mod kw {
    syn::custom_keyword!(disable_fatal_signals);
    syn::custom_keyword!(none);
    syn::custom_keyword!(sigterm_only);
    syn::custom_keyword!(all);
    syn::custom_keyword!(worker_threads);
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
        workers: LitInt,
    },
}

pub enum DisableFatalSignals {
    Default(Token![default]),
    None(kw::none),
    SigtermOnly(kw::sigterm_only),
    All(kw::all),
}

impl Parse for Arg {
    fn parse(input: ParseStream) -> Result<Self> {
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
