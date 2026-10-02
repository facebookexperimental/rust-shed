/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

//! Locating the output of [`TokenStream`]-based parsers, e.g. for error reporting.

use std::marker::PhantomData;

use token_location::Loc;
use token_location::SourceLocation;
use winnow::Parser;

use crate::TokenStream;

/// The parser returned by [`SourceLocationParser::with_location`]
pub struct WithLocation<'i, Token, Output, Error, P> {
    parser: P,
    _marker: PhantomData<ParseFn<'i, Token, Output, Error>>,
}

type ParseFn<'i, Token, Output, Error> = fn(&mut TokenStream<'i, Token>) -> Result<Output, Error>;

/// Extension trait to locate the output of any parser of a [`TokenStream`]
pub trait SourceLocationParser<'i, Token, Output, Error>:
    Parser<TokenStream<'i, Token>, Output, Error> + Sized
{
    /// Wraps the parser to return its output as a [`Loc`], see [`with_location`].
    fn with_location(self) -> WithLocation<'i, Token, Output, Error, Self> {
        WithLocation {
            parser: self,
            _marker: PhantomData,
        }
    }
}

impl<'i, Token, Output, Error, P> SourceLocationParser<'i, Token, Output, Error> for P where
    P: Parser<TokenStream<'i, Token>, Output, Error>
{
}

impl<'i, Token, Output, Error, P> Parser<TokenStream<'i, Token>, Loc<Output>, Error>
    for WithLocation<'i, Token, Output, Error, P>
where
    P: Parser<TokenStream<'i, Token>, Output, Error>,
{
    fn parse_next(&mut self, input: &mut TokenStream<'i, Token>) -> Result<Loc<Output>, Error> {
        with_location(&mut self.parser, input)
    }
}

/// Runs `parser`, returning its output as a [`Loc`] spanning the tokens it consumed, excluding any
/// whitespace & comments around them. A parser that consumed nothing is located at the next token.
pub fn with_location<'i, Token, Output, Error, P>(
    parser: &mut P,
    input: &mut TokenStream<'i, Token>,
) -> Result<Loc<Output>, Error>
where
    P: Parser<TokenStream<'i, Token>, Output, Error>,
{
    let start = input.next_token_start();
    let output = parser.parse_next(input)?;
    // A parser that consumed nothing yields an empty location rather than an inverted one.
    let end = input.prev_token_end().max(start);
    Ok(Loc::new(output, SourceLocation::new(start, end)))
}

#[cfg(test)]
mod tests {
    use token_location::IntoLoc;
    use winnow::LocatingSlice;
    use winnow::ascii::multispace0;
    use winnow::combinator::alt;
    use winnow::combinator::opt;
    use winnow::stream::Location as _;

    use super::*;

    #[derive(Copy, Clone, Debug, PartialEq, Eq)]
    enum Token {
        Foo,
        Bar,
    }

    fn tokenize(input: &mut LocatingSlice<&str>) -> Option<Loc<Token>> {
        let _ = multispace0::<_, ()>.parse_next(input);
        let start = input.current_token_start();
        let token = alt::<_, _, (), _>(("foo".value(Token::Foo), "bar".value(Token::Bar)))
            .parse_next(input)
            .ok()?;
        Some(token.at(start..input.current_token_start()))
    }

    fn foo(input: &mut TokenStream<'_, Token>) -> Result<Token, ()> {
        let mut peek = *input;
        match peek.next() {
            Some(token) if token.node == Token::Foo => {
                *input = peek;
                Ok(token.node)
            }
            _ => Err(()),
        }
    }

    fn any_token(input: &mut TokenStream<'_, Token>) -> Result<Token, ()> {
        input.next().map(|token| token.node).ok_or(())
    }

    #[test]
    fn locations_exclude_surrounding_trivia() {
        let mut input = TokenStream::new("  foo\n\nbar  ", tokenize);
        let first = any_token.with_location().parse_next(&mut input).unwrap();
        assert_eq!(first, Token::Foo.at(2..5));
        let second = any_token.with_location().parse_next(&mut input).unwrap();
        assert_eq!(second, Token::Bar.at(7..10));
    }

    #[test]
    fn empty_match_is_located_at_the_next_token() {
        let mut input = TokenStream::new("foo  bar", tokenize);
        any_token.parse_next(&mut input).unwrap();
        let absent = opt(foo).with_location().parse_next(&mut input).unwrap();
        assert_eq!(absent, None.at(5..5));
    }
}
