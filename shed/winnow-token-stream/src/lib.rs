/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! # winnow-token-stream
//!
//! `winnow-token-stream` turns a [`Tokenizer`] function into a [`winnow::stream::Stream`] of
//! tokens, lexed from a string on demand, so parsers can be written against tokens instead of
//! characters & whitespace.
//!
//! ```
//! use winnow_token_stream::TokenStream;
//! use winnow_token_stream::list_of;
//! use winnow_token_stream::token_location::IntoLoc;
//! use winnow_token_stream::token_location::Loc;
//! use winnow_token_stream::winnow::LocatingSlice;
//! use winnow_token_stream::winnow::Parser;
//! use winnow_token_stream::winnow::ascii::digit1;
//! use winnow_token_stream::winnow::ascii::multispace0;
//! use winnow_token_stream::winnow::error::ContextError;
//! use winnow_token_stream::winnow::error::ErrMode;
//! use winnow_token_stream::winnow::error::ParserError;
//! use winnow_token_stream::winnow::stream::Location;
//! use winnow_token_stream::winnow::token::any;
//!
//! #[derive(Clone, Copy, Debug, PartialEq)]
//! enum Token<'i> {
//!     Number(&'i str),
//!     Symbol(char),
//! }
//!
//! type Input<'i> = TokenStream<'i, Token<'i>>;
//!
//! // Lexes numbers & single-character symbols, skipping whitespace.
//! fn tokenize<'i>(input: &mut LocatingSlice<&'i str>) -> Option<Loc<Token<'i>>> {
//!     multispace0::<_, ()>.parse_next(input).ok()?;
//!     let start = input.current_token_start();
//!     let token = match digit1::<_, ()>.parse_next(input) {
//!         Ok(digits) => Token::Number(digits),
//!         Err(()) => Token::Symbol(any::<_, ()>.parse_next(input).ok()?),
//!     };
//!     Some(token.at(start..input.current_token_start()))
//! }
//!
//! fn number(input: &mut Input<'_>) -> Result<u32, ErrMode<ContextError>> {
//!     match input.next().map(|token| token.node) {
//!         Some(Token::Number(digits)) => Ok(digits.parse().unwrap()),
//!         _ => Err(ErrMode::from_input(input)),
//!     }
//! }
//!
//! fn symbol<'i>(expected: char) -> impl Parser<Input<'i>, (), ErrMode<ContextError>> {
//!     move |input: &mut Input<'i>| match input.next().map(|token| token.node) {
//!         Some(Token::Symbol(c)) if c == expected => Ok(()),
//!         _ => Err(ErrMode::from_input(input)),
//!     }
//! }
//!
//! let mut input = TokenStream::new("[1, 2 ,3]", tokenize);
//! let list = list_of(symbol('['), number, symbol(','), symbol(']'))
//!     .parse_next(&mut input)
//!     .unwrap();
//! let (numbers, trailing_comma): (Vec<u32>, bool) = list.node;
//! assert_eq!(numbers, [1, 2, 3]);
//! assert!(!trailing_comma);
//! assert_eq!(list.location, (0..9).into());
//! ```

#![deny(warnings, missing_docs, clippy::all, rustdoc::broken_intra_doc_links)]

mod list_of;
mod location;
mod stream;
mod yank;

pub use list_of::*;
pub use location::*;
pub use stream::*;
/// The location types in this crate's API, so users don't have to keep a matching version of
/// `token_location` as a separate dependency.
pub use token_location;
/// The parser-combinator library this crate's API is built on, so users don't have to keep a
/// matching version of `winnow` as a separate dependency.
pub use winnow;
pub use yank::*;
