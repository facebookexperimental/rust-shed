/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Utilities for yanking textual content up to a certain point in the input stream
//! (e.g. yanking the remainder of the current line)

use std::fmt::Debug;

use winnow::error::ErrMode;
use winnow::error::ParserError;
use winnow::stream::ContainsToken;
use winnow::stream::Stream;

use crate::TokenStream;

/// Skips to the next newline in the input, discard any tokens on the current line
#[inline]
pub fn skip_line_remainder<Token>(input: &mut TokenStream<'_, Token>) {
    // Always skip one token
    let first_on_new_line = input.trivia_before_next().contains('\n');
    if input.next().is_none() || first_on_new_line {
        return;
    }

    while !input.trivia_before_next().contains('\n') && input.next().is_some() {}
}

/// Yanks the remainder of the current line from the input
#[inline]
pub fn yank_line_remainder<'i, Token, Error>(
    input: &mut TokenStream<'i, Token>,
) -> Result<&'i str, ErrMode<Error>>
where
    Token: Debug,
    Error: ParserError<TokenStream<'i, Token>>,
{
    let rem = consumed_text(input, skip_line_remainder);
    if rem.is_empty() {
        return Err(ErrMode::Backtrack(Error::from_input(input)));
    }
    Ok(rem.strip_suffix('\n').unwrap_or(rem))
}

/// The source text of the tokens `skip` consumes, excluding any trivia around them
fn consumed_text<'i, Token>(
    input: &mut TokenStream<'i, Token>,
    skip: impl FnOnce(&mut TokenStream<'i, Token>),
) -> &'i str {
    let text = input.remaining();
    let offset = input.prev_token_end();
    let start = input.next_token_start() - offset;
    skip(input);
    let end = (input.prev_token_end() - offset).max(start);
    &text[start..end]
}

/// Skip to the provided token in the input, discard any tokens in between
#[inline]
pub fn skip_to<Token: Debug>(
    input: &mut TokenStream<'_, Token>,
    token_match: impl ContainsToken<Token>,
) {
    loop {
        let checkpoint = input.checkpoint();
        let Some(token) = input.next() else {
            return;
        };
        if token_match.contains_token(token.node) {
            input.reset(&checkpoint);
            return;
        }
    }
}

/// Yank up to the provided token in the input
#[inline]
pub fn yank_to<'i, Token, Error>(
    input: &mut TokenStream<'i, Token>,
    token_match: impl ContainsToken<Token>,
) -> Result<&'i str, ErrMode<Error>>
where
    Token: Debug,
    Error: ParserError<TokenStream<'i, Token>>,
{
    let content = consumed_text(input, |input| skip_to(input, token_match));
    if content.is_empty() {
        return Err(ErrMode::Backtrack(Error::from_input(input)));
    }
    Ok(content)
}

#[cfg(test)]
mod tests {
    use token_location::IntoLoc;
    use token_location::Loc;
    use winnow::LocatingSlice;
    use winnow::Parser;
    use winnow::ascii::multispace0;
    use winnow::combinator::alt;
    use winnow::stream::Location as _;

    use super::*;

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum Token {
        Foo,
        Bar,
        Baz,
        Dash,
        Equals,
    }

    fn tokenize(input: &mut LocatingSlice<&str>) -> Option<Loc<Token>> {
        let _ = multispace0::<_, ()>.parse_next(input);
        let start = input.current_token_start();
        let token = alt::<_, _, (), _>((
            '-'.value(Token::Dash),
            '='.value(Token::Equals),
            "foo".value(Token::Foo),
            "bar".value(Token::Bar),
            "baz".value(Token::Baz),
        ))
        .parse_next(input)
        .ok()?;
        Some(token.at(start..input.current_token_start()))
    }

    type TestInput<'i> = TokenStream<'i, Token>;

    /// Allows parsers matching on [`Token`] during parsing
    impl<'i> winnow::Parser<TestInput<'i>, Token, ()> for Token {
        fn parse_next(&mut self, input: &mut TestInput<'i>) -> winnow::Result<Token, ()> {
            let token = input
                .next()
                .ok_or_else(|| ParserError::from_input(input))?
                .node;
            if *self == token {
                Ok(token)
            } else {
                Err(ParserError::from_input(input))
            }
        }
    }

    impl ContainsToken<Token> for Token {
        fn contains_token(&self, token: Token) -> bool {
            *self == token
        }
    }

    #[test]
    fn yank_remainder_of_line() {
        let raw_input = "foo-bar=baz\nbaz=bar-foo\n";
        let mut input = TestInput::new(raw_input, tokenize);
        (Token::Foo, Token::Dash).parse_next(&mut input).unwrap();
        assert_eq!(yank_line_remainder::<_, ()>(&mut input).unwrap(), "bar=baz");

        (Token::Baz, Token::Equals, Token::Bar)
            .parse_next(&mut input)
            .unwrap();
        assert_eq!(yank_line_remainder::<_, ()>(&mut input).unwrap(), "-foo");
    }

    #[test]
    fn yank_up_to_token() {
        let raw_input = "foo-bar=baz\nbaz=bar-foo\n";
        let mut input = TestInput::new(raw_input, tokenize);
        assert_eq!(
            yank_to::<_, ()>(&mut input, Token::Equals).unwrap(),
            "foo-bar"
        );
        (Token::Equals).parse_next(&mut input).unwrap();
        assert_eq!(
            yank_to::<_, ()>(&mut input, Token::Dash).unwrap(),
            "baz\nbaz=bar"
        );
    }
}
