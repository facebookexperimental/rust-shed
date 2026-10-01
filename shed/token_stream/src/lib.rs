/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! # token_stream
//!
//! `token_stream` turns a [`Tokenizer`] function into a [`winnow::stream::Stream`] of tokens,
//! lexed from a string on demand.
//!
//! This allows consumers to use [`winnow`]'s parser-combinator infrastructure using tokens instead of raw strings/chars, e.g.
//! ```ignore
//! (
//!     keyword::VAR,
//!     TokenType::identifier,
//!     TokenType::Equals,
//!     expr,
//!     TokenType::Semicolon,
//! )
//!     .parse_next(input)
//! ```
//! Compared to the raw string matching equivalent:
//! ```ignore
//! (
//!     "var".with_span(),
//!     multispace1,
//!     identifier_parser.with_span(),
//!     multispace1,
//!     '=',
//!     multispace1,
//!     expr.with_span(),
//!     multispace1,
//!     ';',
//! )
//! ```
//! This becomes exponentionally more complex the more you have to account for whitespace & optional sequences.

mod list_of;
mod location;
mod stream;
mod yank;

pub use list_of::*;
pub use location::*;
pub use stream::*;
pub use yank::*;
